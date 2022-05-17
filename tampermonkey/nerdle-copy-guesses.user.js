// ==UserScript==
// @name         Wordle Copy Guesses - Nerdle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/nerdle
// @version      0.1
// @description  Provide a button to quickly copy Nerdle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://nerdlegame.com/
// @match        https://bi.nerdlegame.com/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=nerdlegame.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function getMaxGuessCount() {
        if (window.location.hostname === "nerdlegame.com") {
            return 6;
        } else if (window.location.hostname === "bi.nerdlegame.com") {
            return 7;
        } else {
            return null;
        }
    }

    function getCorrectSolutions(gameState) {
        if (window.location.hostname === "nerdlegame.com") {
            // single string; array-ify
            return [gameState.solution];
        } else if (window.location.hostname === "bi.nerdlegame.com") {
            // already an array
            return gameState.solution;
        } else {
            return null;
        }
    }

    function setUpButton(navBar) {
        var lastNavButton = navBar.children[navBar.children.length - 1];

        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guess";
        copyButton.style.border = "1px solid #000";
        copyButton.style.marginLeft = "1em";
        navBar.insertBefore(copyButton, lastNavButton);

        copyButton.addEventListener("click", function () {
            var gameState = JSON.parse(window.localStorage.gameState);
            var guessList = gameState.guesses;
            var i, j;

            if (guessList.length === getMaxGuessCount()) {
                // potentially defeated; check that
                var correctSolutions = getCorrectSolutions(gameState);
                var isSolutionCorrect = [];
                for (i = 0; i < correctSolutions.length; i++) {
                    isSolutionCorrect.push(false);
                }

                for (i = 0; i < guessList.length; i++) {
                    for (j = 0; j < correctSolutions.length; j++) {
                        if (guessList[i] === correctSolutions[j]) {
                            isSolutionCorrect[j] = true;
                        }
                    }
                }

                // append correct solutions for puzzles where defeated
                for (i = 0; i < correctSolutions.length; i++) {
                    if (!isSolutionCorrect[i]) {
                        guessList.push(correctSolutions[i]);
                    }
                }
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
        var navBar = document.querySelector(".pb-nav");
        if (navBar === null && navBarCount < 5) {
            navBarCount += 1;
            window.setTimeout(waitAndGiveNavBar, 200);
        }
        setUpButton(navBar);
    };
    window.setTimeout(waitAndGiveNavBar, 200);
})();
