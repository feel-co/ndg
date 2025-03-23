document.addEventListener("DOMContentLoaded", function () {
  const searchInput = document.getElementById("search-input");
  if (!searchInput) return;

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
});
