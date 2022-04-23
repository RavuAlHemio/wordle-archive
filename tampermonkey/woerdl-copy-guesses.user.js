// ==UserScript==
// @name         Wordle Copy Guesses - Wördl (wordle.at)
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/woerdl
// @version      0.1
// @description  Provide a button to quickly copy Wördl guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://wordle.at/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=wordle.at
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(statsButton) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        statsButton.parentNode.insertBefore(copyButton, statsButton);

        copyButton.addEventListener("click", function () {
            var gameData = JSON.parse(window.localStorage.data);
            var guessList = gameData.lastStartedGame.tries
                .map(function (s) { return s.join(""); });
            var solution = gameData.lastStartedGame.gameData.solution.join("");
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

    var waitAndGiveStatsButton;
    var statsButtonCount = 0;
    waitAndGiveStatsButton = function () {
        var statsButton = document.getElementById("openStats");
        if (statsButton === null && statsButtonCount < 5) {
            statsButtonCount += 1;
            window.setTimeout(waitAndGiveStatsButton, 200);
        }
        setUpButton(statsButton);
    };
    window.setTimeout(waitAndGiveStatsButton, 200);
})();
