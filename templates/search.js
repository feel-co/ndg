document.addEventListener("DOMContentLoaded", function () {
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

          // Simple search implementation
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
      });
  }

  // Options filter
  // FIXME: this is here as a temporary solution, and should be removed
  // to its own file as soon as possible.
  const optionsFilter = document.getElementById("options-filter");
  if (optionsFilter) {
    const optionsContainer = document.querySelector(".options-container");
    if (!optionsContainer) return;

    // Create filter results counter
    const filterResults = document.createElement("div");
    filterResults.className = "filter-results";
    optionsFilter.parentNode.insertBefore(
      filterResults,
      optionsFilter.nextSibling,
    );

    function filterOptions() {
      const searchTerm = optionsFilter.value.toLowerCase().trim();
      const options = document.querySelectorAll(".option");
      let visibleCount = 0;
      const totalCount = options.length;

      options.forEach(function (option) {
        // Get text content from all relevant parts
        const nameElem = option.querySelector(".option-name");
        const descriptionElem = option.querySelector(".option-description");
        const optionId = option.id ? option.id.toLowerCase() : "";

        const name = nameElem ? nameElem.textContent.toLowerCase() : "";
        const description = descriptionElem
          ? descriptionElem.textContent.toLowerCase()
          : "";

        // Also search in any other text within the option
        const allText = option.textContent.toLowerCase();

        if (
          searchTerm === "" ||
          name.includes(searchTerm) ||
          description.includes(searchTerm) ||
          optionId.includes(searchTerm) ||
          allText.includes(searchTerm)
        ) {
          option.style.display = "";
          visibleCount++;
        } else {
          option.style.display = "none";
        }
      });

      // Update results counter
      if (searchTerm) {
        filterResults.textContent = `Showing ${visibleCount} of ${totalCount} options`;
        filterResults.style.display = "block";
      } else {
        filterResults.style.display = "none";
      }
    }

    // Set up event listeners
    optionsFilter.addEventListener("input", filterOptions);
    optionsFilter.addEventListener("change", filterOptions);

    // Allow clearing with Escape key
    optionsFilter.addEventListener("keydown", function (e) {
      if (e.key === "Escape") {
        optionsFilter.value = "";
        filterOptions();
      }
    });

    // Initially trigger filter if there's a value
    if (optionsFilter.value) {
      filterOptions();
    }
  }
});
