// ==UserScript==
// @name         Wordle Copy Guesses - Wordle.cz
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/cz
// @version      0.1
// @description  Provide a button to quickly copy Wordle.cz guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://www.wordle.cz/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=wordle.cz
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    var letterWithoutDiacritics = {
        "\u00C1": "A",
        "\u010C": "C",
        "\u010E": "D",
        "\u00C9": "E",
        "\u011A": "E",
        "\u00CD": "I",
        "\u0147": "N",
        "\u00D3": "O",
        "\u0158": "R",
        "\u0160": "S",
        "\u0164": "T",
        "\u00DA": "U",
        "\u016E": "U",
        "\u00DD": "Y",
        "\u017D": "Z",
    };

    function setUpButton(gameField) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        gameField.appendChild(copyButton);

        copyButton.addEventListener("click", function () {
            var guessesWithoutDiacritics = JSON.parse(window.localStorage["wordle.board"])
                .map(function (g) { return g.join("").toUpperCase(); });

            // Wordle.cz stores the solution as a global variable :-D
            var solution = window.word.toUpperCase();

            // restore diacritics
            var guessList = [];
            var i, j;
            for (i = 0; i < guessesWithoutDiacritics.length; i++) {
                var thisGuess = "";
                for (j = 0; j < guessesWithoutDiacritics[i].length; j++) {
                    var g = guessesWithoutDiacritics[i].charAt(j);
                    var s = solution.charAt(j);
                    var swd = letterWithoutDiacritics[s];
                    if (swd !== undefined) {
                        if (swd === g) {
                            thisGuess += s;
                            continue;
                        }
                    }

                    thisGuess += g;
                }
                guessList.push(thisGuess);
            }
            if (window.solved === "lost") {
                // append correct solution
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

    var waitAndGiveGameField;
    var gameFieldCount = 0;
    waitAndGiveGameField = function () {
        var gameField = document.getElementById("klavesnice");
        if (gameField === null && gameFieldCount < 5) {
            gameFieldCount += 1;
            window.setTimeout(waitAndGiveGameField, 200);
        }
        setUpButton(gameField);
    };
    window.setTimeout(waitAndGiveGameField, 200);
})();
