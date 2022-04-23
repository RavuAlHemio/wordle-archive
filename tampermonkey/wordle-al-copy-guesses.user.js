// ==UserScript==
// @name         Wordle Copy Guesses - Wordle.al
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/al
// @version      0.1
// @description  Provide a button to quickly copy Wordle.al guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://wordle.al/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=wordle.al
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(sectionsDiv) {
        var copyButtonDiv = document.createElement("div");
        copyButtonDiv.style.textAlign = "center";

        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.border = "1px solid #000";
        copyButton.style.padding = "0.2em";

        sectionsDiv.appendChild(copyButtonDiv);
        copyButtonDiv.appendChild(copyButton);

        copyButton.addEventListener("click", function () {
            var guessList = JSON.parse(window.localStorage.gameState)
                .board
                .map(function (s) { return s.toUpperCase(); });
            if (guessList.length === 6) {
                // possible defeat (hard to obtain correct solution...)
                // add a hint
                guessList.push("<correct solution here if defeated>");
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveSectionsDiv;
    var sectionsDivCount = 0;
    waitAndGiveSectionsDiv = function () {
        var sectionsDiv = document.querySelector("#__next div.container");
        if (sectionsDiv === null && sectionsDivCount < 5) {
            sectionsDivCount += 1;
            window.setTimeout(waitAndGiveSectionsDiv, 200);
        }
        setUpButton(sectionsDiv);
    };
    window.setTimeout(waitAndGiveSectionsDiv, 200);
})();
