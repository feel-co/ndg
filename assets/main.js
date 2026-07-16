// Polyfill for requestIdleCallback for Safari and unsupported browsers
if (typeof window.requestIdleCallback === "undefined") {
  window.requestIdleCallback = function (cb) {
    const start = Date.now();
    const idlePeriod = 50;
    return setTimeout(function () {
      cb({
        didTimeout: false,
        timeRemaining: function () {
          return Math.max(0, idlePeriod - (Date.now() - start));
        },
      });
    }, 1);
  };
  window.cancelIdleCallback = function (id) {
    clearTimeout(id);
  };
}

// Create mobile elements if they don't exist
function createMobileElements() {
  const mobileToggle = document.createElement("button");
  mobileToggle.className = "mobile-sidebar-toggle";
  mobileToggle.type = "button";
  mobileToggle.setAttribute("aria-label", "Open contents");
  mobileToggle.setAttribute("aria-controls", "mobile-sidebar");
  mobileToggle.setAttribute("aria-expanded", "false");
  mobileToggle.innerHTML = `
    <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <line x1="3" y1="12" x2="21" y2="12"></line>
      <line x1="3" y1="6" x2="21" y2="6"></line>
      <line x1="3" y1="18" x2="21" y2="18"></line>
    </svg>
  `;

  function updateMobileToggleVisibility() {
    const header = document.querySelector("header");
    if (window.innerWidth > 800) {
      if (mobileToggle.parentNode) {
        mobileToggle.parentNode.removeChild(mobileToggle);
      }
    } else {
      if (header && !header.contains(mobileToggle)) {
        header.insertBefore(mobileToggle, header.firstChild);
      }
    }
  }
  updateMobileToggleVisibility();
  window.addEventListener("resize", updateMobileToggleVisibility);

  const mobileBackdrop = document.createElement("div");
  mobileBackdrop.className = "mobile-sidebar-backdrop";
  mobileBackdrop.hidden = true;

  const mobileContainer = document.createElement("div");
  mobileContainer.id = "mobile-sidebar";
  mobileContainer.className = "mobile-sidebar-container";
  mobileContainer.setAttribute("role", "dialog");
  mobileContainer.setAttribute("aria-modal", "true");
  mobileContainer.setAttribute("aria-labelledby", "mobile-sidebar-title");
  mobileContainer.setAttribute("aria-hidden", "true");
  mobileContainer.innerHTML = `
    <div class="mobile-sidebar-header">
      <h2 id="mobile-sidebar-title">Menu</h2>
      <button type="button" class="mobile-sidebar-close" aria-label="Close contents">&times;</button>
    </div>
    <nav class="mobile-sidebar-site-nav" aria-label="Site navigation"></nav>
    <div class="mobile-sidebar-content">
      <!-- Sidebar content will be cloned here -->
    </div>
  `;

  // Create mobile search popup
  const mobileSearchPopup = document.createElement("div");
  mobileSearchPopup.id = "mobile-search-popup";
  mobileSearchPopup.className = "mobile-search-popup";
  mobileSearchPopup.setAttribute("role", "dialog");
  mobileSearchPopup.setAttribute("aria-modal", "true");
  mobileSearchPopup.setAttribute("aria-label", "Search");
  mobileSearchPopup.innerHTML = `
    <div class="mobile-search-container" role="document">
      <div class="mobile-search-header">
        <input type="search" id="mobile-search-input" placeholder="Search..." aria-label="Search" autocomplete="off" />
        <button type="button" id="close-mobile-search" class="close-mobile-search" aria-label="Close search">&times;</button>
      </div>
      <div id="mobile-search-results" class="mobile-search-results" role="region" aria-live="polite" aria-label="Search results"></div>
    </div>
  `;

  // Insert at end of body so it is not affected by .container flex or stacking context
  document.body.appendChild(mobileBackdrop);
  document.body.appendChild(mobileContainer);
  document.body.appendChild(mobileSearchPopup);

  // Immediately populate mobile sidebar content if desktop sidebar exists
  const desktopSidebar = document.querySelector(".sidebar");
  const mobileSidebarContent = mobileContainer.querySelector(
    ".mobile-sidebar-content",
  );
  if (desktopSidebar && mobileSidebarContent) {
    mobileSidebarContent.innerHTML = desktopSidebar.innerHTML;
  }

  const headerNav = document.querySelector(".header-nav ul");
  const mobileSiteNav = mobileContainer.querySelector(
    ".mobile-sidebar-site-nav",
  );
  if (headerNav && mobileSiteNav) {
    mobileSiteNav.innerHTML = headerNav.outerHTML;
  }
}

// Highlight search terms on target pages
function highlightTextInContent(container, terms) {
  if (!container || !terms || terms.length === 0) return;

  // Create a case-insensitive regex pattern
  const pattern = terms
    .map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("|");
  const regex = new RegExp(`(${pattern})`, "gi");

  // Elements to skip highlighting
  const skipTags = new Set(["SCRIPT", "STYLE", "CODE", "PRE", "MARK"]);

  function highlightNode(node) {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent;
      // Use match instead of test to avoid regex state issues
      if (text.match(regex)) {
        const span = document.createElement("span");
        // Create a fresh regex for replace to avoid state issues
        const replaceRegex = new RegExp(`(${pattern})`, "gi");
        span.innerHTML = text.replace(
          replaceRegex,
          '<mark class="search-highlight">$1</mark>',
        );
        node.replaceWith(...Array.from(span.childNodes));
      }
    } else if (
      node.nodeType === Node.ELEMENT_NODE &&
      !skipTags.has(node.tagName)
    ) {
      Array.from(node.childNodes).forEach(highlightNode);
    }
  }

  highlightNode(container);

  // Scroll to first highlight after a brief delay
  setTimeout(() => {
    const firstHighlight = container.querySelector(".search-highlight");
    if (firstHighlight) {
      firstHighlight.scrollIntoView({ behavior: "smooth", block: "center" });
      firstHighlight.classList.add("search-highlight-active");
    }
  }, 100);
}

// Initialize scroll spy
function initScrollSpy() {
  const pageToc = document.querySelector(".page-toc");
  if (!pageToc) return;

  const tocLinks = pageToc.querySelectorAll(".page-toc-list a");
  const content = document.querySelector(".content");
  if (!tocLinks.length || !content) return;

  const headings = Array.from(
    content.querySelectorAll("h1[id], h2[id], h3[id]"),
  );

  if (!headings.length) return;

  // Build ordered (heading, tocLink) pairs. Using a queue per ID handles
  // duplicate heading text correctly: the first TOC link for a given href
  // is paired with the first heading carrying that ID in document order, the
  // second TOC link with the second heading, etc.
  const headingQueues = new Map();
  headings.forEach((h) => {
    if (!headingQueues.has(h.id)) headingQueues.set(h.id, []);
    headingQueues.get(h.id).push(h);
  });

  const pairs = [];
  tocLinks.forEach((link) => {
    const href = link.getAttribute("href");
    if (!href || !href.startsWith("#")) return;
    const id = href.slice(1);
    const queue = headingQueues.get(id);
    if (queue?.length) {
      pairs.push({ heading: queue.shift(), link });
    }
  });

  // Ensure pairs are sorted by document position in case the TOC order ever
  // diverges from heading order.
  pairs.sort(
    (a, b) => headings.indexOf(a.heading) - headings.indexOf(b.heading),
  );

  let activeLink = null;

  // Update active link based on scroll position
  function updateActiveLink() {
    const threshold = 120; // threshold from the top of the viewport

    let currentPair = null;

    // Find the last heading that is at or above the threshold
    for (const pair of pairs) {
      const rect = pair.heading.getBoundingClientRect();
      if (rect.top <= threshold) {
        currentPair = pair;
      }
    }

    // If no heading is above threshold, use first heading if it's in view
    if (!currentPair && pairs.length > 0) {
      const firstRect = pairs[0].heading.getBoundingClientRect();
      if (firstRect.top < window.innerHeight) {
        currentPair = pairs[0];
      }
    }

    const newLink = currentPair?.link ?? null;

    if (newLink !== activeLink) {
      if (activeLink) {
        activeLink.classList.remove("active");
      }
      if (newLink) {
        newLink.classList.add("active");
      }
      activeLink = newLink;
    }
  }

  // Scroll event handler
  let ticking = false;
  function onScroll() {
    if (!ticking) {
      requestAnimationFrame(() => {
        updateActiveLink();
        ticking = false;
      });
      ticking = true;
    }
  }

  window.addEventListener("scroll", onScroll, { passive: true });

  // Also update on hash change (direct link navigation)
  window.addEventListener("hashchange", () => {
    requestAnimationFrame(updateActiveLink);
  });

  // Set initial active state after a small delay to ensure
  // browser has completed any hash-based scrolling
  setTimeout(updateActiveLink, 100);
}

function initMobileNavigation() {
  const mobileSidebarContainer = document.querySelector(
    ".mobile-sidebar-container",
  );
  const mobileSidebarToggle = document.querySelector(".mobile-sidebar-toggle");
  const mobileSidebarBackdrop = document.querySelector(
    ".mobile-sidebar-backdrop",
  );
  const mobileSidebarClose = document.querySelector(".mobile-sidebar-close");
  const mobileSidebarLinks = mobileSidebarContainer?.querySelectorAll("a") ?? [];

  if (!mobileSidebarToggle || !mobileSidebarContainer || !mobileSidebarBackdrop)
    return;

  const openMobileSidebar = () => {
    mobileSidebarContainer.classList.add("active");
    mobileSidebarBackdrop.hidden = false;
    mobileSidebarBackdrop.classList.add("active");
    mobileSidebarToggle.setAttribute("aria-expanded", "true");
    mobileSidebarContainer.setAttribute("aria-hidden", "false");
    document.body.classList.add("mobile-sidebar-open");
    mobileSidebarClose?.focus();
  };

  const closeMobileSidebar = () => {
    mobileSidebarContainer.classList.remove("active");
    mobileSidebarBackdrop.classList.remove("active");
    mobileSidebarToggle.setAttribute("aria-expanded", "false");
    mobileSidebarContainer.setAttribute("aria-hidden", "true");
    document.body.classList.remove("mobile-sidebar-open");
    setTimeout(() => {
      if (!mobileSidebarBackdrop.classList.contains("active")) {
        mobileSidebarBackdrop.hidden = true;
      }
    }, 200);
  };

  mobileSidebarToggle.addEventListener("click", (e) => {
    e.stopPropagation();
    if (mobileSidebarContainer.classList.contains("active")) {
      closeMobileSidebar();
    } else {
      openMobileSidebar();
    }
  });

  mobileSidebarBackdrop.addEventListener("click", closeMobileSidebar);
  mobileSidebarClose?.addEventListener("click", closeMobileSidebar);
  mobileSidebarLinks.forEach((link) => {
    link.addEventListener("click", closeMobileSidebar);
  });
  window.addEventListener("resize", () => {
    if (window.innerWidth > 800) {
      closeMobileSidebar();
    }
  });

  document.addEventListener("keydown", (event) => {
    if (
      event.key === "Escape" &&
      mobileSidebarContainer.classList.contains("active")
    ) {
      closeMobileSidebar();
    }
  });
}

function getFilterMatches(searchTerm, originalOrder, data) {
  if (searchTerm === "") {
    return originalOrder.map((element, index) => ({ element, index }));
  }

  const terms = searchTerm.split(/\s+/).filter(Boolean);
  const firstTerm = terms[0] || "";

  return data
    .filter((item) => terms.every((term) => item.searchText.includes(term)))
    .sort((a, b) => {
      const aRank = a.name.includes(firstTerm) ? 0 : 1;
      const bRank = b.name.includes(firstTerm) ? 0 : 1;
      if (aRank !== bRank) return aRank - bRank;
      const aPos = a.name.includes(firstTerm)
        ? a.name.indexOf(firstTerm)
        : a.searchText.indexOf(firstTerm);
      const bPos = b.name.includes(firstTerm)
        ? b.name.indexOf(firstTerm)
        : b.searchText.indexOf(firstTerm);
      return aPos - bPos || a.index - b.index;
    });
}

function reconcileFilteredItems({
  container,
  hiddenContainer,
  matches,
  data,
  reduceMotion,
  isCurrentRun,
}) {
  const visibleElements = new Set(matches.map((item) => item.element));
  const leaving = [];

  for (const item of data) {
    if (visibleElements.has(item.element)) continue;
    if (reduceMotion.matches || !container.contains(item.element)) {
      hiddenContainer.content.appendChild(item.element);
    } else {
      item.element.classList.add("filter-leaving");
      leaving.push(item.element);
    }
  }

  const updateVisibleItems = () => {
    if (!isCurrentRun()) return;
    for (const element of leaving) {
      element.classList.remove("filter-leaving");
      if (!visibleElements.has(element)) {
        hiddenContainer.content.appendChild(element);
      }
    }

    const entering = [];
    let reference = container.firstChild;
    for (const item of matches) {
      const wasHidden = !container.contains(item.element);
      if (wasHidden && !reduceMotion.matches) {
        item.element.classList.add("filter-entering");
        entering.push(item.element);
      }

      if (item.element === reference) {
        reference = reference.nextSibling;
      } else {
        container.insertBefore(item.element, reference);
      }
    }

    if (entering.length > 0) {
      requestAnimationFrame(() => {
        for (const element of entering) {
          element.classList.remove("filter-entering");
        }
      });
    }
  };

  if (leaving.length > 0) {
    setTimeout(updateVisibleItems, 160);
  } else {
    updateVisibleItems();
  }
}

function setupListFilter({
  inputId,
  containerSelector,
  itemSelector,
  nameSelector,
  noun,
}) {
  const input = document.getElementById(inputId);
  const container = document.querySelector(containerSelector);
  if (!input || !container) return;

  const hiddenContainer = document.createElement("template");
  document.body.appendChild(hiddenContainer);

  const filterResults = document.createElement("div");
  filterResults.className = "filter-results";
  input.parentNode.insertBefore(filterResults, input.nextSibling);

  const isMobile =
    window.innerWidth < 768 || /Mobi|Android/i.test(navigator.userAgent);
  const items = Array.from(document.querySelectorAll(itemSelector));
  const totalCount = items.length;
  const originalOrder = items.slice();
  const data = items.map((element, index) => {
    const name = element.querySelector(nameSelector)?.textContent ?? "";
    return {
      element,
      index,
      name: name.toLowerCase(),
      searchText:
        `${element.id || ""} ${element.textContent || ""}`.toLowerCase(),
    };
  });

  let lastTerm = "";
  let timeout = null;
  let filterRun = 0;
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  const applyFilter = () => {
    const searchTerm = input.value.toLowerCase().trim();
    if (lastTerm === searchTerm) return;
    lastTerm = searchTerm;
    filterRun += 1;
    const currentRun = filterRun;
    for (const item of data) {
      item.element.classList.remove("filter-entering", "filter-leaving");
    }

    const matches = getFilterMatches(searchTerm, originalOrder, data);
    reconcileFilteredItems({
      container,
      hiddenContainer,
      matches,
      data,
      reduceMotion,
      isCurrentRun: () => currentRun === filterRun,
    });

    if (searchTerm !== "" && matches.length < totalCount) {
      filterResults.textContent = `Showing ${matches.length} of ${totalCount} ${noun}`;
      filterResults.style.display = "block";
    } else {
      filterResults.style.display = "none";
    }
  };

  const debounce = () => {
    clearTimeout(timeout);
    timeout = setTimeout(applyFilter, isMobile ? 200 : 100);
  };

  input.addEventListener("input", debounce);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      input.value = "";
      applyFilter();
    }
  });
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden && input.value) applyFilter();
  });

  if (input.value) applyFilter();
  if (isMobile && totalCount > 50) {
    requestIdleCallback(() => {
      const height = items[0]?.offsetHeight ?? 0;
      if (height > 0) {
        items.forEach((item) => {
          item.style.containIntrinsicSize = `0 ${height}px`;
        });
      }
    });
  }
}

document.addEventListener("DOMContentLoaded", function () {
  // Apply sidebar state immediately before DOM rendering
  try {
    if (localStorage.getItem("sidebar-collapsed") === "true") {
      document.documentElement.classList.add("sidebar-collapsed");
      document.body.classList.add("sidebar-collapsed");
    }
  } catch {
    // localStorage unavailable
  }

  if (!document.querySelector(".mobile-sidebar-toggle")) {
    createMobileElements();
  }
  initMobileNavigation();

  // Initialize scroll spy for page TOC
  initScrollSpy();

  // Template container for collapsed sidebar content (prevents Ctrl+F from finding hidden content)
  const sidebarHiddenContainer = document.createElement("template");

  // Handle sidebar section toggles - move content to template when collapsed
  document
    .querySelectorAll(".sidebar-section > .sidebar-section-content")
    .forEach((content) => {
      const details = content.parentElement;
      const toggleContent = () => {
        if (details.hasAttribute("open")) {
          // Section opened - move content back to DOM
          if (sidebarHiddenContainer.content.contains(content)) {
            const summary = details.querySelector("summary");
            details.insertBefore(
              content,
              summary ? summary.nextSibling : details.firstChild,
            );
          }
        } else {
          // Section closed - move content to template (removes from DOM, Ctrl+F won't find it)
          if (content.parentElement === details) {
            sidebarHiddenContainer.content.appendChild(content);
          }
        }
      };

      // Use MutationObserver to detect open/close changes
      const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
          if (mutation.attributeName === "open") {
            toggleContent();
          }
        });
      });

      observer.observe(details, { attributes: true });

      // Initial state check
      if (!details.hasAttribute("open")) {
        sidebarHiddenContainer.content.appendChild(content);
      }
    });

  // Desktop Sidebar Toggle
  const sidebarToggle = document.querySelector(".sidebar-toggle");

  // On page load, sync the state from `documentElement` to `body`
  if (document.documentElement.classList.contains("sidebar-collapsed")) {
    document.body.classList.add("sidebar-collapsed");
  }

  if (sidebarToggle) {
    sidebarToggle.addEventListener("click", function () {
      // Toggle on both elements for consistency
      document.documentElement.classList.toggle("sidebar-collapsed");
      document.body.classList.toggle("sidebar-collapsed");

      // Use documentElement to check state and save to localStorage
      const isCollapsed =
        document.documentElement.classList.contains("sidebar-collapsed");
      try {
        localStorage.setItem("sidebar-collapsed", isCollapsed);
      } catch {
        // localStorage unavailable
      }
    });
  }

  // Make headings clickable for anchor links
  const content = document.querySelector(".content");
  if (content) {
    const headings = content.querySelectorAll("h1, h2, h3, h4, h5, h6");

    headings.forEach(function (heading) {
      // Generate a valid, unique ID for each heading
      if (!heading.id) {
        let baseId = heading.textContent
          .toLowerCase()
          .replace(/[^a-z0-9\s-_]/g, "") // remove invalid chars
          .replace(/^[^a-z]+/, "") // remove leading non-letters
          .replace(/[\s-_]+/g, "-")
          .replace(/^-+|-+$/g, "") // trim leading/trailing dashes
          .trim();
        if (!baseId) {
          baseId = "section";
        }
        let id = baseId;
        let counter = 1;
        while (document.getElementById(id)) {
          id = `${baseId}-${counter++}`;
        }
        heading.id = id;
      }

      // Make the entire heading clickable
      heading.addEventListener("click", function () {
        const id = this.id;
        history.pushState(null, null, "#" + id);

        // Scroll with offset
        const offset = this.getBoundingClientRect().top + window.scrollY - 80;
        window.scrollTo({
          top: offset,
          behavior: "smooth",
        });
      });
    });
  }

  // Process footnotes
  if (content) {
    const footnoteContainer = document.querySelector(".footnotes-container");

    // Find all footnote references and create a footnotes section
    const footnoteRefs = content.querySelectorAll('a[href^="#fn"]');
    if (footnoteRefs.length > 0) {
      const footnotesDiv = document.createElement("div");
      footnotesDiv.className = "footnotes";

      const footnotesHeading = document.createElement("h2");
      footnotesHeading.textContent = "Footnotes";
      footnotesDiv.appendChild(footnotesHeading);

      const footnotesList = document.createElement("ol");
      footnoteContainer.appendChild(footnotesDiv);
      footnotesDiv.appendChild(footnotesList);

      // Add footnotes
      document.querySelectorAll(".footnote").forEach((footnote) => {
        const id = footnote.id;
        const content = footnote.innerHTML;

        const li = document.createElement("li");
        li.id = id;
        li.innerHTML = content;

        // Add backlink
        const backlink = document.createElement("a");
        backlink.href = "#fnref:" + id.replace("fn:", "");
        backlink.className = "footnote-backlink";
        backlink.textContent = "↩";
        li.appendChild(backlink);

        footnotesList.appendChild(li);
      });
    }
  }

  // Copy link functionality
  document.querySelectorAll(".copy-link").forEach(function (copyLink) {
    copyLink.addEventListener("click", function (e) {
      e.preventDefault();
      e.stopPropagation();

      // Get option ID from parent element
      const option = copyLink.closest(".option");
      const optionId = option.id;

      // Create URL with hash
      const url = new URL(window.location.href);
      url.hash = optionId;

      // Copy to clipboard
      navigator.clipboard
        .writeText(url.toString())
        .then(function () {
          // Show feedback
          const feedback = copyLink.nextElementSibling;
          feedback.style.display = "inline";

          // Hide after 2 seconds
          setTimeout(function () {
            feedback.style.display = "none";
          }, 2000);
        })
        .catch(function (err) {
          console.error("Could not copy link: ", err);
        });
    });
  });

  // Handle initial hash navigation
  function scrollToElement(element) {
    if (element) {
      const offset = element.getBoundingClientRect().top + window.scrollY - 80;
      window.scrollTo({
        top: offset,
        behavior: "smooth",
      });
    }
  }

  if (window.location.hash) {
    const targetElement = document.getElementById(
      decodeURIComponent(window.location.hash.slice(1)),
    );
    if (targetElement) {
      setTimeout(() => scrollToElement(targetElement), 0);
      // Add highlight class for options page
      if (targetElement.classList.contains("option")) {
        targetElement.classList.add("highlight");
      }
    }
  }

  const optionsIndexList = document.querySelector(".options-index-list");
  setupListFilter({
    inputId: "options-filter",
    containerSelector: optionsIndexList
      ? ".options-index-list"
      : ".options-container",
    itemSelector: optionsIndexList ? ".option-page-row" : ".option",
    nameSelector: optionsIndexList ? ".option-page-title" : ".option-name",
    noun: optionsIndexList ? "option groups" : "options",
  });

  setupListFilter({
    inputId: "lib-filter",
    containerSelector: ".lib-container",
    itemSelector: ".lib-entry",
    nameSelector: ".lib-entry-name",
    noun: "functions",
  });

  // URL-based search highlighting
  const urlParams = new URLSearchParams(window.location.search);
  const highlightQuery = urlParams.get("highlight");
  if (highlightQuery && content) {
    // Simple tokenizer that doesn't depend on search engine
    const queryTerms = highlightQuery
      .toLowerCase()
      .trim()
      .split(/\s+/)
      .filter((term) => term.length >= 2); // min 2 chars like search engine

    if (queryTerms.length > 0) {
      highlightTextInContent(content, queryTerms);
    }
  }
});
