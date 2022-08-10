// ==UserScript==
// @name         Wordle Copy Guesses - Sweardle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/sweardle
// @version      0.1
// @description  Provide a button to quickly copy Sweardle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://sweardle.com/*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=sweardle.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function getCookie(key) {
        var pieces = document.cookie.split(';');
        for (var i = 0; i < pieces.length; i++) {
            var equalsIndex = pieces[i].indexOf("=");
            var singleKey = pieces[i].substring(0, equalsIndex).trim();
            if (key === singleKey) {
                var singleValue = pieces[i].substring(equalsIndex + 1).trim();
                return singleValue;
            }
        }
        return null;
    }

    function isSuccess(scores) {
        var succeeded = false;
        for (var r = 0; r < scores.length; r++) {
            succeeded = true;
            for (var c = 0; c < scores[r].length; c++) {
                if (scores[r][c] !== 2) {
                    // not a green square
                    succeeded = false;
                    break;
                }
            }
            if (succeeded) {
                break;
            }
        }
        return succeeded;
    }

    function setUpButtons(keyboard) {
        var buttonBar = document.createElement("div");
        buttonBar.style.textAlign = "center";
        keyboard.parentElement.insertBefore(buttonBar, keyboard);

        var copyResultButton = document.createElement("input");
        copyResultButton.type = "button";
        copyResultButton.value = "copy result";
        buttonBar.appendChild(copyResultButton);

        var copyGuessesButton = document.createElement("input");
        copyGuessesButton.type = "button";
        copyGuessesButton.value = "copy guesses";
        buttonBar.appendChild(copyGuessesButton);

        copyResultButton.addEventListener("click", function () {
            var guessData = JSON.parse(getCookie("guessData"));
            var dayNumber = guessData.lastDay;

            var succeeded = isSuccess(guessData.scores);
            var guessField = "";
            for (var r = 0; r < guessData.scores.length; r++) {
                if (guessField.length > 0) {
                    guessField += "\n";
                }

                for (var c = 0; c < guessData.scores[r].length; c++) {
                    switch (guessData.scores[r][c]) {
                        case 0: guessField += "\u2B1B"; break; // black square
                        case 1: guessField += "\uD83D\uDFE8"; break; // yellow square
                        case 2: guessField += "\uD83D\uDFE9"; break; // green square
                    }
                }
            }

            var finalText = "Sweardle day " + dayNumber + " ";
            if (succeeded) {
                finalText += "" + guessData.scores.length;
            } else {
                finalText += "X";
            }
            finalText += "/4\n";
            finalText += guessField;
            finalText += "\n\nsweardle.com";

            navigator.clipboard.writeText(finalText).then(function () {
                copyResultButton.value = "copied!";
            }, function () {
                copyResultButton.value = "failed";
            });
        });

        copyGuessesButton.addEventListener("click", function () {
            var guessData = JSON.parse(getCookie("guessData"));
            var guessList = guessData.guesses;
            var guesses = guessList.join("\n");

            if (!isSuccess(guessData.scores)) {
                guesses += "\n<correct solution here>";
            }

            navigator.clipboard.writeText(guesses).then(function () {
                copyGuessesButton.value = "copied!";
            }, function () {
                copyGuessesButton.value = "failed";
            });
        });
    }

    var waitAndGiveKeyboard;
    var keyboardCount = 0;
    waitAndGiveKeyboard = function () {
        var keyboard = document.querySelector("body > div.keyboard");
        if (keyboard === null && keyboardCount < 5) {
            keyboardCount += 1;
            window.setTimeout(waitAndGiveKeyboard, 200);
        }
        setUpButtons(keyboard);
    };
    window.setTimeout(waitAndGiveKeyboard, 200);
})();
