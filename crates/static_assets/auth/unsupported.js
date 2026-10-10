// Popup dismissal for the unsupported-browser page only. ES5 on purpose.
//
// auth.js uses syntax those browsers reject, and a parse error drops that whole file.
// This page is what they are sent to, so its popup handler lives here. The retained
// incompatible-browser template loads it after auth.js; each classic script is parsed alone.
(function () {
  var BOTTOM = 90;

  function openPopups() {
    return document.querySelectorAll("details[data-controller~='popup'][open]");
  }

  function isPopup(list) {
    var controller = list.getAttribute("data-controller") || "";

    return (" " + controller + " ").indexOf(" popup ") !== -1;
  }

  function orient(list) {
    var menu = list.querySelector("[data-popup-target='menu']");
    var topClass;
    var rect;

    if (!menu) return;

    topClass = list.getAttribute("data-popup-orientation-top-class");

    // A reopen keeps the upward class from last time. Measuring in that state reports the
    // flipped box, and the menu can stay off the top of the window. Drop it, then measure.
    if (topClass) list.classList.remove(topClass);

    rect = menu.getBoundingClientRect();

    if (topClass) {
      if (window.innerHeight - rect.bottom < BOTTOM) list.classList.add(topClass);
    }

    menu.style.setProperty("--max-width", window.innerWidth - rect.left + "px");
  }

  document.addEventListener("click", function (event) {
    var open = openPopups();
    var i;

    for (i = 0; i < open.length; i++) {
      if (!open[i].contains(event.target)) open[i].open = false;
    }
  });

  document.addEventListener("keydown", function (event) {
    var open;
    var i;
    var summary;

    if (event.key !== "Escape" && event.keyCode !== 27) return;

    open = openPopups();

    for (i = 0; i < open.length; i++) {
      open[i].open = false;
      summary = open[i].querySelector("summary");

      if (summary) summary.focus();
    }
  });

  document.addEventListener(
    "toggle",
    function (event) {
      var opened = event.target;
      var open;
      var i;

      if (!opened || opened.tagName !== "DETAILS" || !opened.open || !isPopup(opened)) return;

      orient(opened);
      open = openPopups();

      for (i = 0; i < open.length; i++) {
        if (open[i] !== opened) open[i].open = false;
      }
    },
    true,
  );
})();
