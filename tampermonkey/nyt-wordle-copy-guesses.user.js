// ==UserScript==
// @name         Wordle Copy Guesses - NYT
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/nyt
// @version      0.1
// @description  Provide a button to quickly copy Wordle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://www.nytimes.com/games/wordle/*
// @icon         https://www.nytimes.com/games/wordle/images/NYT-Wordle-Icon-32.png
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(navBar) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.backgroundColor = "var(--key-bg)";
        copyButton.style.color = "var(--key-text-color)";
        navBar.insertBefore(copyButton, navBar.firstChild);

        copyButton.addEventListener("click", function () {
            var wordleState = JSON.parse(window.localStorage["nyt-wordle-state"]);
            var guessList = wordleState.boardState
                .filter(function (s) { return s !== ''; })
                .map(function (s) { return s.toUpperCase(); });
            var solution = wordleState.solution.toUpperCase();
            if (guessList.length === 6 && guessList[guessList.length - 1] !== solution) {
                // defeated; also append correct solution
                guessList.push(solution);
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveNavBar;
    var navBarCount = 0;
    waitAndGiveNavBar = function () {
        var navBar = document.querySelector(".wordle-app-header div[class^=AppHeader-module_menuRight__]");
        if (navBar === null && navBarCount < 5) {
            navBarCount += 1;
            window.setTimeout(waitAndGiveNavBar, 200);
        }
        setUpButton(navBar);
    };
    window.setTimeout(waitAndGiveNavBar, 200);
})();
