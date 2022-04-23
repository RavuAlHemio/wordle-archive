// ==UserScript==
// @name         Wordle Copy Guesses - Duotrigordle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/wordle32
// @version      0.1
// @description  Provide a button to quickly copy Duotrigordle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://duotrigordle.com/
// @match        https://duotrigordle.ro/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=duotrigordle.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(headerBar) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        headerBar.insertBefore(copyButton, headerBar.lastChild);

        copyButton.addEventListener("click", function () {
            var state = JSON.parse(window.localStorage["duotrigordle-state"]);
            var guessList = state.guesses;
            // add hint for missed solutions in case we lost
            guessList.push("<missed solutions here>");
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveHeaderBar;
    var headerBarCount = 0;
    waitAndGiveHeaderBar = function () {
        var headerBar = document.querySelector("#root .game .header .row-2");
        if (headerBar === null && headerBarCount < 5) {
            headerBarCount += 1;
            window.setTimeout(waitAndGiveHeaderBar, 200);
        }
        setUpButton(headerBar);
    };
    window.setTimeout(waitAndGiveHeaderBar, 200);
})();
