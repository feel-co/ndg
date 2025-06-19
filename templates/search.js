document.addEventListener("DOMContentLoaded", function () {
  // Desktop Sidebar Toggle
  const sidebarToggle = document.querySelector(".sidebar-toggle");
  if (sidebarToggle) {
    sidebarToggle.addEventListener("click", function () {
      document.body.classList.toggle("sidebar-collapsed");
      const isCollapsed = document.body.classList.contains("sidebar-collapsed");
      localStorage.setItem("sidebar-collapsed", isCollapsed);
    });
  }

  const searchInput = document.getElementById("search-input");
  if (searchInput) {
    const searchResults = document.getElementById("search-results");

    // Load search data
    fetch("assets/search-data.json")
      .then((response) => {
        if (!response.ok) {
          throw new Error("Failed to load search data");
        }
        return response.json();
      })
      .then((data) => {
        searchInput.addEventListener("input", function () {
          const searchTerm = this.value.toLowerCase().trim();

          if (searchTerm.length < 2) {
            searchResults.innerHTML = "";
            searchResults.style.display = "none";
            return;
          }

          // "Simple" search implementation
          const results = data
            .filter(
              (doc) =>
                doc.title.toLowerCase().includes(searchTerm) ||
                doc.content.toLowerCase().includes(searchTerm),
            )
            .slice(0, 10);

          if (results.length > 0) {
            searchResults.innerHTML = results
              .map(
                (doc) => `
                          <div class="search-result-item">
                              <a href="${doc.path}">${doc.title}</a>
                          </div>
                      `,
              )
              .join("");
            searchResults.style.display = "block";
          } else {
            searchResults.innerHTML =
              '<div class="search-result-item">No results found</div>';
            searchResults.style.display = "block";
          }
        });

        // Hide results when clicking outside
        document.addEventListener("click", function (event) {
          if (
            !searchInput.contains(event.target) &&
            !searchResults.contains(event.target)
          ) {
            searchResults.style.display = "none";
          }
        });

        // Focus search when pressing slash key
        document.addEventListener("keydown", function (event) {
          if (event.key === "/" && document.activeElement !== searchInput) {
            event.preventDefault();
            searchInput.focus();
          }
        });
      })
      .catch((error) => {
        console.error("Error loading search data:", error);
        // Create fallback empty search data so search doesn't break
        // 2025-04-05: raf was an idiot and this became necessary.
        window.searchData = [];
        searchInput.addEventListener("input", function() {
          const searchTerm = this.value.toLowerCase().trim();
          if (searchTerm.length < 2) {
            searchResults.innerHTML = "";
            searchResults.style.display = "none";
          } else {
            searchResults.innerHTML = '<div class="search-result-item">No results found</div>';
            searchResults.style.display = "block";
          }
        });
      });
  }

  // Mobile search functionality - detect mobile and add click behavior
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
      // On desktop, let the normal click behavior work (focus the input)
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
  let mobileSearchPopup = document.getElementById("mobile-search-popup");
  let mobileSearchInput = document.getElementById("mobile-search-input");
  let mobileSearchResults = document.getElementById("mobile-search-results");
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
    if (mobileSearchPopup && mobileSearchPopup.classList.contains("active") &&
        !mobileSearchPopup.contains(event.target) &&
        !searchInput.contains(event.target)) {
      closeMobileSearch();
    }
  });

  // Close mobile search on escape key
  document.addEventListener("keydown", function(event) {
    if (event.key === "Escape" && mobileSearchPopup && mobileSearchPopup.classList.contains("active")) {
      closeMobileSearch();
    }
  });

  // Mobile search input functionality (reuse search data if available)
  if (mobileSearchInput && mobileSearchResults) {
    // Load search data for mobile search
    fetch("assets/search-data.json")
      .then((response) => {
        if (!response.ok) {
          throw new Error("Failed to load search data");
        }
        return response.json();
      })
      .then((data) => {
        mobileSearchInput.addEventListener("input", function () {
          const searchTerm = this.value.toLowerCase().trim();

          if (searchTerm.length < 2) {
            mobileSearchResults.innerHTML = "";
            mobileSearchResults.style.display = "none";
            return;
          }

          // "Simple" search implementation
          const results = data
            .filter(
              (doc) =>
                doc.title.toLowerCase().includes(searchTerm) ||
                doc.content.toLowerCase().includes(searchTerm),
            )
            .slice(0, 10);

          if (results.length > 0) {
            mobileSearchResults.innerHTML = results
              .map(
                (doc) => `
                          <div class="search-result-item">
                              <a href="${doc.path}">${doc.title}</a>
                          </div>
                      `,
              )
              .join("");
            mobileSearchResults.style.display = "block";
          } else {
            mobileSearchResults.innerHTML =
              '<div class="search-result-item">No results found</div>';
            mobileSearchResults.style.display = "block";
          }
        });
      })
      .catch((error) => {
        console.error("Error loading search data for mobile:", error);
        mobileSearchInput.addEventListener("input", function() {
          const searchTerm = this.value.toLowerCase().trim();
          if (searchTerm.length < 2) {
            mobileSearchResults.innerHTML = "";
            mobileSearchResults.style.display = "none";
          } else {
            mobileSearchResults.innerHTML = '<div class="search-result-item">No results found</div>';
            mobileSearchResults.style.display = "block";
          }
        });
      });
  }

  // Handle window resize to update mobile behavior
  window.addEventListener('resize', function() {
    // Close mobile search if window is resized to desktop size
    if (!isMobile() && mobileSearchPopup && mobileSearchPopup.classList.contains("active")) {
      closeMobileSearch();
    }
  });

  // Options filter
  const optionsFilter = document.getElementById("options-filter");
  if (optionsFilter) {
    const optionsContainer = document.querySelector(".options-container");
    if (!optionsContainer) return;

    const styleEl = document.createElement('style');
    styleEl.textContent = '.option-hidden{display:none!important}';
    document.head.appendChild(styleEl);

    // Create filter results counter
    const filterResults = document.createElement("div");
    filterResults.className = "filter-results";
    optionsFilter.parentNode.insertBefore(
      filterResults,
      optionsFilter.nextSibling,
    );

    // Detect if we're on a mobile device
    // Possibly the worst way of doing this...
    const isMobile = window.innerWidth < 768 || /Mobi|Android/i.test(navigator.userAgent);

    // Cache all option elements and their searchable content
    const options = Array.from(document.querySelectorAll(".option"));
    const totalCount = options.length;

    // Pre-process and optimize searchable content
    const optionsData = options.map(option => {
      const nameElem = option.querySelector(".option-name");
      const descriptionElem = option.querySelector(".option-description");
      const id = option.id ? option.id.toLowerCase() : "";
      const name = nameElem ? nameElem.textContent.toLowerCase() : "";
      const description = descriptionElem ? descriptionElem.textContent.toLowerCase() : "";

      // Extract keywords for faster searching
      const keywords = (id + " " + name + " " + description)
        .toLowerCase()
        .split(/\s+/)
        .filter(word => word.length > 1);

      return {
        element: option,
        id,
        name,
        description,
        keywords,
        searchText: (id + " " + name + " " + description).toLowerCase()
      };
    });

    // Chunk size and rendering variables
    const CHUNK_SIZE = isMobile ? 15 : 40;
    let pendingRender = null;
    let currentChunk = 0;
    let itemsToProcess = [];

    function debounce(func, wait) {
      let timeout;
      return function() {
        const context = this;
        const args = arguments;
        clearTimeout(timeout);
        timeout = setTimeout(() => func.apply(context, args), wait);
      };
    }

    // Process options in chunks to prevent UI freezing
    function processNextChunk() {
      const startIdx = currentChunk * CHUNK_SIZE;
      const endIdx = Math.min(startIdx + CHUNK_SIZE, itemsToProcess.length);

      if (startIdx < itemsToProcess.length) {
        // Process current chunk
        for (let i = startIdx; i < endIdx; i++) {
          const item = itemsToProcess[i];
          if (item.visible) {
            item.element.classList.remove('option-hidden');
          } else {
            item.element.classList.add('option-hidden');
          }
        }

        currentChunk++;
        pendingRender = requestAnimationFrame(processNextChunk);
      } else {
        // Finished processing all chunks
        pendingRender = null;
        currentChunk = 0;
        itemsToProcess = [];

        // Update counter at the very end for best performance
        if (filterResults.visibleCount !== undefined) {
          if (filterResults.visibleCount < totalCount) {
            filterResults.textContent = `Showing ${filterResults.visibleCount} of ${totalCount} options`;
            filterResults.style.display = "block";
          } else {
            filterResults.style.display = "none";
          }
        }
      }
    }

    // Highly optimized filter function
    function filterOptions() {
      const searchTerm = optionsFilter.value.toLowerCase().trim();

      // Cancel any pending renders
      if (pendingRender) {
        cancelAnimationFrame(pendingRender);
        pendingRender = null;
      }

      // Reset
      currentChunk = 0;
      itemsToProcess = [];

      // Fast path for empty search
      if (searchTerm === "") {
        // On very large datasets, still use chunks for showing all
        if (totalCount > 200) {
          itemsToProcess = options.map(element => ({ element, visible: true }));
          filterResults.visibleCount = totalCount;
          pendingRender = requestAnimationFrame(processNextChunk);
        } else {
          // For smaller datasets, batch update without chunking
          options.forEach(option => option.classList.remove('option-hidden'));
          filterResults.style.display = "none";
        }
        return;
      }

      // Prepare search and split into terms for better matching
      const searchTerms = searchTerm.split(/\s+/).filter(term => term.length > 0);
      let visibleCount = 0;

      // Optimize based on term count
      if (searchTerms.length === 1) {
        // Single term search - common case
        const term = searchTerms[0];

        itemsToProcess = optionsData.map(data => {
          // First check exact matches on id/name (most common use case)
          let visible = data.id.includes(term) || data.name.includes(term);

          // Only check the full text if we don't have a match yet
          if (!visible) {
            visible = data.searchText.includes(term);
          }

          if (visible) visibleCount++;
          return { element: data.element, visible };
        });
      } else {
        itemsToProcess = optionsData.map(data => {
          const visible = searchTerms.every(term =>
            data.searchText.includes(term)
          );

          if (visible) visibleCount++;
          return { element: data.element, visible };
        });
      }

      // Store count for later use
      filterResults.visibleCount = visibleCount;

      // Process in chunks
      pendingRender = requestAnimationFrame(processNextChunk);
    }

    // Use different debounce times for desktop vs mobile
    const debouncedFilter = debounce(filterOptions, isMobile ? 200 : 100);

    // Set up event listeners
    optionsFilter.addEventListener("input", debouncedFilter);
    optionsFilter.addEventListener("change", filterOptions);

    // Allow clearing with Escape key
    optionsFilter.addEventListener("keydown", function (e) {
      if (e.key === "Escape") {
        optionsFilter.value = "";
        filterOptions(); // Apply immediately without debounce
      }
    });

    // Handle visibility changes to improve perceived performance
    document.addEventListener('visibilitychange', function() {
      if (!document.hidden && optionsFilter.value) {
        filterOptions();
      }
    });

    // Initially trigger filter if there's a value
    if (optionsFilter.value) {
      filterOptions();
    }

    // Pre-calculate heights for smoother scrolling (this should prevent layout thrashing)
    if (isMobile && totalCount > 50) {
      requestIdleCallback(() => {
        const sampleOption = options[0];
        if (sampleOption) {
          const height = sampleOption.offsetHeight;
          if (height > 0) {
            options.forEach(opt => {
              opt.style.containIntrinsicSize = `0 ${height}px`;
            });
          }
        }
      });
    }
  }

  // Mobile Sidebar Functionality
  // This functionality has been moved to main.js
});
