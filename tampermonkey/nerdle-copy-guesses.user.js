// ==UserScript==
// @name         Wordle Copy Guesses - Nerdle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/nerdle
// @version      0.1
// @description  Provide a button to quickly copy Nerdle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://nerdlegame.com/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=nerdlegame.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

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
