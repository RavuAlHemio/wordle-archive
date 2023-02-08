// ==UserScript==
// @name         Wordle Copy Guesses - Globle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/globle
// @version      0.1
// @description  Provide a button to quickly copy "Globle" and "Globle: Capitals" guesses into the clipboard.
// @author       Paul Staroch <paul@staroch.name>
// @match        https://globle-game.com/game
// @match        https://globle-capitals.com/game
// @icon         https://www.google.com/s2/favicons?sz=64&domain=globle-game.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(keyboard) {
        var copyButtonDiv = document.createElement("div");
        copyButtonDiv.style.textAlign = "center";

        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.color = "black";
        copyButton.style.border = "solid 1px black";
        keyboard.parentNode.prepend(copyButtonDiv);
        copyButtonDiv.appendChild(copyButton);

        copyButton.addEventListener("click", function () {
            var gameState = JSON.parse(window.localStorage.guesses);
            var guessList;
            if ('countries' in gameState) {
                guessList = gameState.countries;
            }
            else {
                guessList = gameState.cities;
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveStatsButton;
    var buttonCount = 0;
    waitAndGiveStatsButton = function () {
        var buttonDiv = null;
        var panelDivs = document.querySelectorAll("button[aria-label='Statistics']");
        if (panelDivs.length > 0) {
            buttonDiv = panelDivs[panelDivs.length - 1];
        }
        if (buttonDiv === null && buttonCount < 5) {
            buttonCount += 1;
            window.setTimeout(waitAndGiveStatsButton, 200);
        }
        setUpButton(buttonDiv);
    };
    window.setTimeout(waitAndGiveStatsButton, 200);
})();

