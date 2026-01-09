if (!window.searchNamespace) window.searchNamespace = {};

class SearchEngine {
  constructor() {
    this.documents = [];
    this.tokenMap = new Map();
    this.isLoaded = false;
    this.loadError = false;
    this.fullDocuments = null; // for lazy loading
    this.rootPath = window.searchNamespace?.rootPath || "";
  }

  // Check if we can use Web Worker
  get useWebWorker() {
    if (searchWorker === false) return false; // previously failed
    const worker = initializeSearchWorker();
    return worker !== null;
  }

  // Load search data from JSON
  async loadData() {
    if (this.isLoaded && !this.loadError) return;

    // Clear previous error state on retry
    this.loadError = false;

    try {
      // Load JSON data, try multiple possible paths
      // FIXME: There is only one possible path for now, and this search data is guaranteed
      // to generate at this location, but we'll want to extend this in the future.
      const possiblePaths = [
        `${this.rootPath}assets/search-data.json`,
        "/assets/search-data.json", // fallback for root-level sites
      ];

      let response = null;
      let usedPath = "";

      for (const path of possiblePaths) {
        try {
          const testResponse = await fetch(path);
          if (testResponse.ok) {
            response = testResponse;
            usedPath = path;
            break;
          }
        } catch {
          // Continue to next path
        }
      }

      if (!response) {
        throw new Error("Search data file not found at any expected location");
      }

      console.log(`Loading search data from: ${usedPath}`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      // Use our "optimized" JSON parser for large files
      const documents = await this.parseJSONChunked(response);
      if (!Array.isArray(documents)) {
        throw new Error("Invalid search data format");
      }

      this.initializeFromDocuments(documents);
      this.isLoaded = true;
      console.log(`Loaded ${documents.length} documents for search`);
    } catch (error) {
      console.error("Error loading search data:", error);
      this.documents = [];
      this.tokenMap.clear();
      this.loadError = true;
    }
  }

  // Initialize from documents array
  async initializeFromDocuments(documents) {
    if (!Array.isArray(documents)) {
      console.error("Invalid documents format:", typeof documents);
      this.documents = [];
    } else {
      this.documents = documents;
      console.log(`Initialized with ${documents.length} documents`);
    }
    try {
      await this.buildTokenMap();
    } catch (error) {
      console.error("Error building token map:", error);
    }
  }

  // Initialize from search index structure
  initializeIndex(indexData) {
    this.documents = indexData.documents || [];
    this.tokenMap = new Map(Object.entries(indexData.tokenMap || {}));
  }

  // Build token map
  // This is helpful for faster searching with progressive loading
  buildTokenMap() {
    return new Promise((resolve, reject) => {
      this.tokenMap.clear();

      if (!Array.isArray(this.documents)) {
        console.error("No documents to build token map");
        resolve();
        return;
      }

      const totalDocs = this.documents.length;
      let processedDocs = 0;

      try {
        // Process in chunks to avoid blocking UI
        const processChunk = (startIndex, chunkSize) => {
          try {
            const endIndex = Math.min(startIndex + chunkSize, totalDocs);

            for (let i = startIndex; i < endIndex; i++) {
              const doc = this.documents[i];
              if (
                !doc ||
                typeof doc.title !== "string" ||
                typeof doc.content !== "string"
              ) {
                console.warn(`Invalid document at index ${i}:`, doc);
                continue;
              }

              const tokens = this.tokenize(doc.title + " " + doc.content);
              tokens.forEach((token) => {
                if (!this.tokenMap.has(token)) {
                  this.tokenMap.set(token, []);
                }
                this.tokenMap.get(token).push(i);
              });

              processedDocs++;
            }

            // Update progress and yield control
            if (endIndex < totalDocs) {
              setTimeout(() => processChunk(endIndex, chunkSize), 0);
            } else {
              console.log(
                `Built token map with ${this.tokenMap.size} unique tokens from ${processedDocs} documents`,
              );
              resolve();
            }
          } catch (error) {
            reject(error);
          }
        };

        // Start processing with small chunks
        processChunk(0, 100);
      } catch (error) {
        reject(error);
      }
    });
  }

  // Fuzzy match function.
  // Returns score and match indices
  fuzzyMatch(query, target) {
    const lowerQuery = query.toLowerCase();
    const lowerTarget = target.toLowerCase();

    let queryIdx = 0;
    let targetIdx = 0;
    let score = 0;
    const matches = [];

    while (queryIdx < lowerQuery.length && targetIdx < lowerTarget.length) {
      if (lowerQuery[queryIdx] === lowerTarget[targetIdx]) {
        matches.push(targetIdx);
        score += 10;

        if (queryIdx > 0 && targetIdx > 0) {
          const prevMatchIdx = matches[matches.length - 2];
          const distance = targetIdx - prevMatchIdx - 1;

          if (distance === 1) {
            score += 15;
          } else if (distance === 2) {
            score += 5;
          } else if (distance === 3) {
            score += 2;
          }
        }

        queryIdx++;
        targetIdx++;
      } else {
        targetIdx++;
      }
    }

    if (queryIdx !== lowerQuery.length) {
      return null;
    }

    const lengthRatio = lowerQuery.length / lowerTarget.length;
    score *= lengthRatio;

    if (lowerTarget === lowerQuery) {
      score += 100;
    } else if (lowerTarget.startsWith(lowerQuery)) {
      score += 50;
    } else if (lowerTarget.includes(lowerQuery)) {
      score += 30;
    }

    const maxScore = lowerQuery.length * 15 + 100;
    const normalizedScore = score / maxScore;

    return normalizedScore >= 0.3 ? normalizedScore : null;
  }

  // Levenshtein distance for typo tolerance
  // XXX: This is probably not the best option to do this but it is pretty
  // much the only way I know how to do this. Obviously I've researched how
  // to do fuzzy searching and the above implementation *is* that, but this
  // can be kept as a fallback.
  levenshteinDistance(str1, str2) {
    const m = str1.length;
    const n = str2.length;

    if (m === 0) return n;
    if (n === 0) return m;
    if (Math.abs(m - n) > 3) return 999;

    const dp = Array(m + 1)
      .fill(null)
      .map(() => Array(n + 1).fill(0));

    for (let i = 0; i <= m; i++) dp[i][0] = i;
    for (let j = 0; j <= n; j++) dp[0][j] = j;

    for (let i = 1; i <= m; i++) {
      for (let j = 1; j <= n; j++) {
        if (str1[i - 1] === str2[j - 1]) {
          dp[i][j] = dp[i - 1][j - 1];
        } else {
          dp[i][j] =
            1 +
            Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
        }
      }
    }

    return dp[m][n];
  }

  // Tokenize text into searchable terms
  tokenize(text) {
    const tokens = new Set();
    const words = text.toLowerCase().match(/\b[a-zA-Z0-9_-]+\b/g) || [];

    words.forEach((word) => {
      if (word.length > 2) {
        tokens.add(word);
      }
    });

    return Array.from(tokens);
  }

  // Advanced search with ranking
  async search(query, limit = 10) {
    if (!query.trim()) return [];

    // Wait for data to be loaded
    if (!this.isLoaded) {
      await this.loadData();
    }

    if (!this.isLoaded || this.documents.length === 0) {
      console.log("Search data not available");
      return [];
    }

    const searchTerms = this.tokenize(query);
    const rawQuery = query.toLowerCase();

    if (searchTerms.length === 0 && rawQuery.length < 2) return [];

    const useFuzzySearch = rawQuery.length >= 3;

    const pageMatches = new Map();

    this.documents.forEach((doc, docId) => {
      let match = pageMatches.get(docId);
      if (!match) {
        match = { doc, pageScore: 0, matchingAnchors: [] };
        pageMatches.set(docId, match);
      }

      const lowerTitle = doc.title.toLowerCase();
      const lowerContent = doc.content.toLowerCase();

      if (useFuzzySearch) {
        const fuzzyTitleScore = this.fuzzyMatch(rawQuery, lowerTitle);
        const fuzzyContentScore = this.fuzzyMatch(rawQuery, lowerContent);

        if (fuzzyTitleScore !== null) {
          match.pageScore += fuzzyTitleScore * 100;
        } else {
          const editDist = this.levenshteinDistance(
            rawQuery,
            lowerTitle,
          );
          if (
            editDist <= Math.max(1, Math.floor(rawQuery.length * 0.3)) &&
            editDist < rawQuery.length
          ) {
            const typoScore = (1 - editDist / rawQuery.length) * 50;
            match.pageScore += typoScore;
          }
        }

        if (fuzzyContentScore !== null) {
          match.pageScore += fuzzyContentScore * 30;
        } else {
          const editDist = this.levenshteinDistance(
            rawQuery,
            lowerContent,
          );
          if (
            editDist <= Math.max(1, Math.floor(rawQuery.length * 0.3)) &&
            editDist < rawQuery.length
          ) {
            const typoScore = (1 - editDist / rawQuery.length) * 15;
            match.pageScore += typoScore;
          }
        }
      }

      searchTerms.forEach((term) => {
        if (lowerTitle.includes(term)) {
          match.pageScore += lowerTitle === term ? 20 : 10;
        }
        if (lowerContent.includes(term)) {
          match.pageScore += 2;
        }
      });
    });

    pageMatches.forEach((match) => {
      const doc = match.doc;
      if (!doc.anchors || doc.anchors.length === 0) return;

      doc.anchors.forEach((anchor) => {
        const anchorText = anchor.text.toLowerCase();
        let anchorMatches = false;

        if (useFuzzySearch) {
          const fuzzyScore = this.fuzzyMatch(rawQuery, anchorText);
          if (fuzzyScore !== null && fuzzyScore >= 0.4) {
            anchorMatches = true;
          }
        }

        if (!anchorMatches) {
          searchTerms.forEach((term) => {
            if (anchorText.includes(term)) {
              anchorMatches = true;
            }
          });
        }

        if (anchorMatches) {
          match.matchingAnchors.push(anchor);
        }
      });
    });

    const results = Array.from(pageMatches.values())
      .filter((m) => m.pageScore > 5)
      .sort((a, b) => b.pageScore - a.pageScore)
      .slice(0, limit);

    return results;
  }

  // Generate search preview with highlighting
  generatePreview(content, query, maxLength = 150) {
    const lowerContent = content.toLowerCase();

    let bestIndex = -1;
    let bestScore = 0;
    let bestMatch = "";

    // Find the best match position
    const queryWords = this.tokenize(query);
    queryWords.forEach((word) => {
      const index = lowerContent.indexOf(word);
      if (index !== -1) {
        const score = word.length; // longer words get higher priority
        if (score > bestScore) {
          bestScore = score;
          bestIndex = index;
          bestMatch = word;
        }
      }
    });

    if (bestIndex === -1) {
      return this.escapeHtml(content.slice(0, maxLength)) + "...";
    }

    const start = Math.max(0, bestIndex - 50);
    const end = Math.min(content.length, bestIndex + bestMatch.length + 50);
    let preview = content.slice(start, end);

    if (start > 0) preview = "..." + preview;
    if (end < content.length) preview += "...";

    // Escape HTML first, then highlight
    preview = this.escapeHtml(preview);
    preview = this.highlightTerms(preview, queryWords);

    return preview;
  }

  // Escape HTML to prevent XSS
  // TODO: either handle this in the Rust side of things
  // or improve the robustness of the implementation.
  escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  // Highlight search terms in text
  highlightTerms(text, terms) {
    let highlighted = text;

    // Sort terms by length (longer first) to avoid overlapping highlights
    const sortedTerms = [...terms].sort((a, b) => b.length - a.length);

    sortedTerms.forEach((term) => {
      const regex = new RegExp(`(${this.escapeRegex(term)})`, "gi");
      highlighted = highlighted.replace(regex, "<mark>$1</mark>");
    });

    return highlighted;
  }

  /**
   * Web Worker search for large datasets
   * @param {string} query - Search query
   * @param {number} limit - Maximum results
   * @returns {Promise<Array>} Search results
   */
  searchWithWorker(query, limit) {
    const worker = initializeSearchWorker();
    if (!worker) {
      return this.fallbackSearch(query, limit);
    }

    return new Promise((resolve, reject) => {
      const messageId = `search_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`;
      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error("Web Worker search timeout"));
      }, 5000); // 5 second timeout

      const handleMessage = (e) => {
        if (e.data.messageId !== messageId) return;

        clearTimeout(timeout);
        cleanup();

        if (e.data.type === "results") {
          resolve(e.data.data);
        } else if (e.data.type === "error") {
          reject(new Error(e.data.error || "Unknown worker error"));
        }
      };

      const handleError = (error) => {
        clearTimeout(timeout);
        cleanup();
        reject(error);
      };

      const cleanup = () => {
        worker.removeEventListener("message", handleMessage);
        worker.removeEventListener("error", handleError);
      };

      worker.addEventListener("message", handleMessage);
      worker.addEventListener("error", handleError);

      worker.postMessage({
        messageId,
        type: "search",
        data: { documents: this.documents, query, limit },
      });
    });
  }

  // Escape regex special characters
  escapeRegex(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  // Resolve path relative to current page location
  resolvePath(path) {
    // If path already starts with '/', it's absolute from domain root
    if (path.startsWith("/")) {
      return path;
    }

    // If path starts with '#', it's a fragment on current page
    if (path.startsWith("#")) {
      return path;
    }

    // Prepend root path for relative navigation
    return this.rootPath + path;
  }

  // Slightly optimized JSON parser for large files
  // This is so that the search widgets can ACTUALLY deal with the search data
  // which tends to be massive for medium-sized projects.
  async parseJSONChunked(response) {
    const contentLength = response.headers.get("content-length");

    // For small files, use regular JSON parsing
    if (!contentLength || parseInt(contentLength) < 1024 * 1024) {
      // < 1MB
      return await response.json();
    }

    // For large files, use streaming approach
    console.log(
      `Large search file detected (${contentLength} bytes), using streaming parser`,
    );

    const reader = response.body.getReader();
    const decoder = new TextDecoder("utf-8");
    let buffer = "";

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      // Process in chunks to avoid blocking main thread
      if (buffer.length > 100 * 1024) {
        // 100KB chunks
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }

    return JSON.parse(buffer);
  }

  // Lazy loading for search results
  lazyLoadDocuments(docIds, limit = 10) {
    if (!this.fullDocuments) {
      // Store full documents separately for memory efficiency
      this.fullDocuments = this.documents;
      // Create lightweight index documents
      this.documents = this.documents.map((doc) => ({
        id: doc.id,
        title: doc.title,
        path: doc.path,
      }));
    }

    return docIds.slice(0, limit).map((id) => this.fullDocuments[id]);
  }

  // Fallback search method via simple string matching
  fallbackSearch(query, limit = 10) {
    const lowerQuery = query.toLowerCase();

    const results = this.documents
      .map((doc) => {
        const titleMatch = doc.title.toLowerCase().indexOf(lowerQuery);
        const contentMatch = doc.content.toLowerCase().indexOf(lowerQuery);
        let pageScore = 0;

        if (titleMatch !== -1) {
          pageScore += 10;
          if (doc.title.toLowerCase() === lowerQuery) {
            pageScore += 20;
          }
        }
        if (contentMatch !== -1) {
          pageScore += 2;
        }

        // Find matching anchors
        const matchingAnchors = [];
        if (doc.anchors && doc.anchors.length > 0) {
          doc.anchors.forEach((anchor) => {
            const anchorText = anchor.text.toLowerCase();
            if (anchorText.includes(lowerQuery)) {
              matchingAnchors.push(anchor);
            }
          });
        }

        return { doc, pageScore, matchingAnchors, titleMatch, contentMatch };
      })
      .filter((item) => item.pageScore > 0)
      .sort((a, b) => {
        if (a.pageScore !== b.pageScore) return b.pageScore - a.pageScore;
        if (a.titleMatch !== b.titleMatch) return a.titleMatch - b.titleMatch;
        return a.contentMatch - b.contentMatch;
      })
      .slice(0, limit);

    return results;
  }
}

// Web Worker for background search processing
// Create Web Worker if supported - initialized lazily to use rootPath
let searchWorker = null;

function initializeSearchWorker() {
  if (searchWorker !== null || typeof Worker === "undefined") {
    return searchWorker;
  }

  try {
    const rootPath = window.searchNamespace?.rootPath || "";
    const workerPath = rootPath
      ? `${rootPath}assets/search-worker.js`
      : "/assets/search-worker.js";
    searchWorker = new Worker(workerPath);
    console.log("Web Worker initialized for background search");
    return searchWorker;
  } catch (error) {
    console.warn("Web Worker creation failed, using main thread:", error);
    searchWorker = false; // mark as failed so we don't retry
    return null;
  }
}

// Global search engine instance
window.searchNamespace.engine = new SearchEngine();

// Mobile search timeout for debouncing
let mobileSearchTimeout = null;

document.addEventListener("DOMContentLoaded", function() {
  // Initialize search engine immediately
  window.searchNamespace.engine
    .loadData()
    .then(() => {
      console.log("Search data loaded successfully");
    })
    .catch((error) => {
      console.error("Failed to initialize search:", error);
    });

  // Search page specific functionality
  const searchPageInput = document.getElementById("search-page-input");
  if (searchPageInput) {
    // Set up event listener
    searchPageInput.addEventListener("input", function() {
      performSearch(this.value);
    });

    // Perform search if URL has query
    const params = new URLSearchParams(window.location.search);
    const query = params.get("q");
    if (query) {
      searchPageInput.value = query;
      performSearch(query);
    }
  }

  // Desktop Sidebar Toggle
  const searchInput = document.getElementById("search-input");
  if (searchInput) {
    const searchResults = document.getElementById("search-results");
    const searchContainer = searchInput.closest(".search-container");

    searchInput.addEventListener("input", async function() {
      const searchTerm = this.value.trim();

      if (searchTerm.length < 2) {
        searchResults.innerHTML = "";
        searchResults.style.display = "none";
        if (searchContainer) searchContainer.classList.remove("has-results");
        return;
      }

      // Show loading state
      searchResults.innerHTML =
        '<div class="search-result-item">Loading...</div>';
      searchResults.style.display = "block";
      if (searchContainer) searchContainer.classList.add("has-results");

      try {
        const results = await window.searchNamespace.engine.search(
          searchTerm,
          8,
        );

        if (results.length > 0) {
          searchResults.innerHTML = results
            .map((result) => {
              const { doc, matchingAnchors } = result;
              const queryTerms =
                window.searchNamespace.engine.tokenize(searchTerm);
              const highlightedTitle =
                window.searchNamespace.engine.highlightTerms(
                  doc.title,
                  queryTerms,
                );
              const resolvedPath = window.searchNamespace.engine.resolvePath(
                doc.path,
              );

              // Build page result
              let html = `
                <div class="search-result-item search-result-page">
                  <a href="${resolvedPath}">${highlightedTitle}</a>
                </div>
              `;

              // Add anchor results if any
              if (matchingAnchors && matchingAnchors.length > 0) {
                matchingAnchors.forEach((anchor) => {
                  const highlightedAnchor =
                    window.searchNamespace.engine.highlightTerms(
                      anchor.text,
                      queryTerms,
                    );
                  const anchorPath = `${resolvedPath}#${anchor.id}`;
                  html += `
                    <div class="search-result-item search-result-anchor">
                      <a href="${anchorPath}">${highlightedAnchor}</a>
                    </div>
                  `;
                });
              }

              return html;
            })
            .join("");
          searchResults.style.display = "block";
          if (searchContainer) searchContainer.classList.add("has-results");
        } else {
          searchResults.innerHTML =
            '<div class="search-result-item">No results found</div>';
          searchResults.style.display = "block";
          if (searchContainer) searchContainer.classList.add("has-results");
        }
      } catch (error) {
        console.error("Search error:", error);
        searchResults.innerHTML =
          '<div class="search-result-item">Search unavailable</div>';
        searchResults.style.display = "block";
        if (searchContainer) searchContainer.classList.add("has-results");
      }
    });

    // Hide results when clicking outside
    document.addEventListener("click", function(event) {
      if (
        !searchInput.contains(event.target) &&
        !searchResults.contains(event.target)
      ) {
        searchResults.style.display = "none";
        if (searchContainer) searchContainer.classList.remove("has-results");
      }
    });

    // Focus search when pressing slash key
    document.addEventListener("keydown", function(event) {
      if (event.key === "/" && document.activeElement !== searchInput) {
        event.preventDefault();
        searchInput.focus();
      }

      // Close search results on Escape key
      if (
        event.key === "Escape" &&
        (document.activeElement === searchInput ||
          searchResults.style.display === "block")
      ) {
        searchResults.style.display = "none";
        if (searchContainer) searchContainer.classList.remove("has-results");
        searchInput.blur();
      }
    });
  }

  // Mobile search functionality
  // This detects mobile viewport and adds click behavior
  function isMobile() {
    return window.innerWidth <= 800;
  }

  if (searchInput) {
    // Add mobile search behavior
    searchInput.addEventListener("click", function(e) {
      if (isMobile()) {
        e.preventDefault();
        e.stopPropagation();
        openMobileSearch();
      }
      // On desktop, we let the normal click behavior work (focus the input)
    });

    // Prevent typing on mobile (input should only open popup)
    searchInput.addEventListener("keydown", function(e) {
      if (isMobile()) {
        e.preventDefault();
        openMobileSearch();
      }
    });
  }

  // Mobile search popup functionality
  const mobileSearchPopup = document.getElementById("mobile-search-popup");
  const mobileSearchInput = document.getElementById("mobile-search-input");
  const mobileSearchResults = document.getElementById("mobile-search-results");
  const closeMobileSearchBtn = document.getElementById("close-mobile-search");

  function openMobileSearch() {
    if (mobileSearchPopup) {
      mobileSearchPopup.classList.add("active");
      // Focus the input after a small delay to ensure the popup is visible
      setTimeout(() => {
        if (mobileSearchInput) {
          mobileSearchInput.focus();
        }
      }, 100);
    }
  }

  function closeMobileSearch() {
    if (mobileSearchPopup) {
      mobileSearchPopup.classList.remove("active");
      if (mobileSearchInput) {
        mobileSearchInput.value = "";
      }
      if (mobileSearchResults) {
        mobileSearchResults.innerHTML = "";
        mobileSearchResults.style.display = "none";
      }
    }
  }

  if (closeMobileSearchBtn) {
    closeMobileSearchBtn.addEventListener("click", closeMobileSearch);
  }

  // Close mobile search when clicking outside
  document.addEventListener("click", function(event) {
    if (
      mobileSearchPopup &&
      mobileSearchPopup.classList.contains("active") &&
      !mobileSearchPopup.contains(event.target) &&
      !searchInput.contains(event.target)
    ) {
      closeMobileSearch();
    }
  });

  // Close mobile search on escape key
  document.addEventListener("keydown", function(event) {
    if (
      event.key === "Escape" &&
      mobileSearchPopup &&
      mobileSearchPopup.classList.contains("active")
    ) {
      closeMobileSearch();
    }
  });

  // Mobile search input
  if (mobileSearchInput && mobileSearchResults) {
    function handleMobileSearchInput() {
      clearTimeout(mobileSearchTimeout);
      const searchTerm = mobileSearchInput.value.trim();
      if (searchTerm.length < 2) {
        mobileSearchResults.innerHTML = "";
        mobileSearchResults.style.display = "none";
        return;
      }

      mobileSearchTimeout = setTimeout(async () => {
        // Verify the input still matches before proceeding
        if (mobileSearchInput.value.trim() !== searchTerm) return;

        // Show loading state
        mobileSearchResults.innerHTML =
          '<div class="search-result-item">Loading...</div>';
        mobileSearchResults.style.display = "block";

        try {
          const results = await window.searchNamespace.engine.search(
            searchTerm,
            8,
          );
          // Verify again after async operation
          if (mobileSearchInput.value.trim() !== searchTerm) return;

          if (results.length > 0) {
            mobileSearchResults.innerHTML = results
              .map((result) => {
                const { doc, matchingAnchors } = result;
                const queryTerms =
                  window.searchNamespace.engine.tokenize(searchTerm);
                const highlightedTitle =
                  window.searchNamespace.engine.highlightTerms(
                    doc.title,
                    queryTerms,
                  );
                const resolvedPath = window.searchNamespace.engine.resolvePath(
                  doc.path,
                );

                // Build page result
                let html = `
                  <div class="search-result-item search-result-page">
                    <a href="${resolvedPath}">${highlightedTitle}</a>
                  </div>
                `;

                // Add anchor results if any
                if (matchingAnchors && matchingAnchors.length > 0) {
                  matchingAnchors.forEach((anchor) => {
                    const highlightedAnchor =
                      window.searchNamespace.engine.highlightTerms(
                        anchor.text,
                        queryTerms,
                      );
                    const anchorPath = `${resolvedPath}#${anchor.id}`;
                    html += `
                      <div class="search-result-item search-result-anchor">
                        <a href="${anchorPath}">${highlightedAnchor}</a>
                      </div>
                    `;
                  });
                }

                return html;
              })
              .join("");
            mobileSearchResults.style.display = "block";
          } else {
            mobileSearchResults.innerHTML =
              '<div class="search-result-item">No results found</div>';
            mobileSearchResults.style.display = "block";
          }
        } catch (error) {
          console.error("Mobile search error:", error);
          // Verify once more
          if (mobileSearchInput.value.trim() !== searchTerm) return;
          mobileSearchResults.innerHTML =
            '<div class="search-result-item">Search unavailable</div>';
          mobileSearchResults.style.display = "block";
        }
      }, 300);
    }

    mobileSearchInput.addEventListener("input", handleMobileSearchInput);
  }

  // Handle window resize to update mobile behavior
  window.addEventListener("resize", function() {
    // Close mobile search if window is resized to desktop size
    if (
      !isMobile() &&
      mobileSearchPopup &&
      mobileSearchPopup.classList.contains("active")
    ) {
      closeMobileSearch();
    }
  });
});

async function performSearch(query) {
  query = query.trim();
  const resultsContainer = document.getElementById("search-page-results");

  if (query.length < 2) {
    resultsContainer.innerHTML =
      "<p>Please enter at least 2 characters to search</p>";
    return;
  }

  // Show loading state
  resultsContainer.innerHTML = "<p>Searching...</p>";

  try {
    const results = await window.searchNamespace.engine.search(query, 50);

    // Display results
    if (results.length > 0) {
      let html = '<ul class="search-results-list">';
      const queryTerms = window.searchNamespace.engine.tokenize(query);

      for (const result of results) {
        const { doc, matchingAnchors } = result;
        const highlightedTitle = window.searchNamespace.engine.highlightTerms(
          doc.title,
          queryTerms,
        );
        const preview = window.searchNamespace.engine.generatePreview(
          doc.content,
          query,
        );
        const resolvedPath = window.searchNamespace.engine.resolvePath(
          doc.path,
        );

        // Page result
        html += `<li class="search-result-item search-result-page">
          <a href="${resolvedPath}">
            <div class="search-result-title">${highlightedTitle}</div>
            <div class="search-result-preview">${preview}</div>
          </a>
        </li>`;

        // Anchor results
        if (matchingAnchors && matchingAnchors.length > 0) {
          matchingAnchors.forEach((anchor) => {
            const highlightedAnchor =
              window.searchNamespace.engine.highlightTerms(
                anchor.text,
                queryTerms,
              );
            const anchorPath = `${resolvedPath}#${anchor.id}`;
            html += `<li class="search-result-item search-result-anchor">
              <a href="${anchorPath}">
                <div class="search-result-anchor-text">${highlightedAnchor}</div>
              </a>
            </li>`;
          });
        }
      }
      html += "</ul>";
      resultsContainer.innerHTML = html;
    } else {
      resultsContainer.innerHTML = "<p>No results found</p>";
    }

    // Update URL with query
    const url = new URL(window.location.href);
    url.searchParams.set("q", query);
    window.history.replaceState({}, "", url.toString());
  } catch (error) {
    console.error("Search error:", error);
    resultsContainer.innerHTML = "<p>Search temporarily unavailable</p>";
  }
}
