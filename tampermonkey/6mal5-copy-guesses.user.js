// ==UserScript==
// @name         Wordle Copy Guesses - 6mal5
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/6mal5
// @version      0.1
// @description  Provide a button to quickly copy 6mal5 and wordle.ro guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://6mal5.com/
// @match        https://wordle.ro/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=6mal5.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(keyboard) {
        var copyButtonDiv = document.createElement("div");
        copyButtonDiv.style.textAlign = "center";
        copyButtonDiv.style.marginTop = "1em";

        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.backgroundColor = "rgb(51 65 85)";
        copyButton.style.color = "rgb(255 255 255)";
        copyButton.style.padding = "0.2em";
        keyboard.parentNode.appendChild(copyButtonDiv);
        copyButtonDiv.appendChild(copyButton);

        copyButton.addEventListener("click", function () {
            var gameState = JSON.parse(window.localStorage.gameState);
            var guessList = gameState.guesses;
            if (guessList.length === 6 && guessList[guessList.length - 1] !== gameState.solution) {
                // defeated; also append correct solution
                guessList.push(gameState.solution);
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveKeyboard;
    var keyboardCount = 0;
    waitAndGiveKeyboard = function () {
        var keyboardDiv = null;
        var panelDivs = document.querySelectorAll("#root > div > div");
        if (panelDivs.length > 0) {
            keyboardDiv = panelDivs[panelDivs.length - 1];
        }
        if (keyboardDiv === null && keyboardCount < 5) {
            keyboardCount += 1;
            window.setTimeout(waitAndGiveKeyboard, 200);
        }
        setUpButton(keyboardDiv);
    };
    window.setTimeout(waitAndGiveKeyboard, 200);
})();
