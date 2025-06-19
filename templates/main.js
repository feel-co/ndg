document.addEventListener("DOMContentLoaded", function () {
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
      localStorage.setItem("sidebar-collapsed", isCollapsed);
    });
  }

  // Mobile Sidebar Functionality
  const mobileSidebarContainer = document.getElementById("mobile-sidebar-container");
  const mobileSidebarFab = document.getElementById("mobile-sidebar-fab");
  const mobileSidebarContent = document.getElementById("mobile-sidebar-content");
  const mobileSidebarHandle = document.querySelector(".mobile-sidebar-handle");
  const desktopSidebar = document.getElementById("sidebar");

  if (mobileSidebarContainer && mobileSidebarFab && desktopSidebar) {
    // Clone sidebar content to mobile container
    // Only clone if the mobile sidebar is empty
    if (mobileSidebarContent.innerHTML.trim() === "") {
        mobileSidebarContent.innerHTML = desktopSidebar.innerHTML;
    }

    const openMobileSidebar = () => {
      mobileSidebarContainer.classList.add("active");
      mobileSidebarFab.setAttribute("aria-expanded", "true");
      mobileSidebarContainer.setAttribute("aria-hidden", "false");
    };

    const closeMobileSidebar = () => {
      mobileSidebarContainer.classList.remove("active");
      mobileSidebarFab.setAttribute("aria-expanded", "false");
      mobileSidebarContainer.setAttribute("aria-hidden", "true");
    };

    mobileSidebarFab.addEventListener("click", (e) => {
      e.stopPropagation();
      if (mobileSidebarContainer.classList.contains("active")) {
        closeMobileSidebar();
      } else {
        openMobileSidebar();
      }
    });

    // Drag functionality
    let isDragging = false;
    let startY = 0;
    let startHeight = 0;

    mobileSidebarHandle.addEventListener("mousedown", (e) => {
      isDragging = true;
      startY = e.pageY;
      startHeight = mobileSidebarContainer.offsetHeight;
      mobileSidebarHandle.style.cursor = "grabbing";
      document.body.style.userSelect = "none"; // Prevent text selection
    });

    mobileSidebarHandle.addEventListener("touchstart", (e) => {
      isDragging = true;
      startY = e.touches[0].pageY;
      startHeight = mobileSidebarContainer.offsetHeight;
    });

    document.addEventListener("mousemove", (e) => {
      if (!isDragging) return;
      const deltaY = startY - e.pageY;
      const newHeight = startHeight + deltaY;
      const vh = window.innerHeight;
      const minHeight = vh * 0.15;
      const maxHeight = vh * 0.9;

      if (newHeight >= minHeight && newHeight <= maxHeight) {
        mobileSidebarContainer.style.height = `${newHeight}px`;
      }
    });

    document.addEventListener("touchmove", (e) => {
      if (!isDragging) return;
      const deltaY = startY - e.touches[0].pageY;
      const newHeight = startHeight + deltaY;
      const vh = window.innerHeight;
      const minHeight = vh * 0.15;
      const maxHeight = vh * 0.9;

      if (newHeight >= minHeight && newHeight <= maxHeight) {
        mobileSidebarContainer.style.height = `${newHeight}px`;
      }
    });

    document.addEventListener("mouseup", () => {
      if (isDragging) {
        isDragging = false;
        mobileSidebarHandle.style.cursor = "grab";
        document.body.style.userSelect = "";
      }
    });

    document.addEventListener("touchend", () => {
      isDragging = false;
    });

    // Close on outside click
    document.addEventListener("click", (event) => {
      if (mobileSidebarContainer.classList.contains("active") &&
          !mobileSidebarContainer.contains(event.target) &&
          !mobileSidebarFab.contains(event.target)) {
        closeMobileSidebar();
      }
    });

    // Close on escape key
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && mobileSidebarContainer.classList.contains("active")) {
        closeMobileSidebar();
      }
    });
  }
});
