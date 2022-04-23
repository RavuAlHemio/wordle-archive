// ==UserScript==
// @name         Wordle Copy Guesses - Heardle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/heardle
// @version      0.1
// @description  Provide a button to quickly copy Heardle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://www.heardle.app/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=www.heardle.app
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(statsButton) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.backgroundColor = "var(--color-mg)";
        statsButton.parentElement.insertBefore(copyButton, statsButton);

        copyButton.addEventListener("click", function () {
            var stats = JSON.parse(window.localStorage.userStats);
            var todaysStats = stats[stats.length - 1];
            var guessList = todaysStats.guessList
                .map(function (g) { return (g.answer === undefined) ? "" : g.answer; });
            var victory = todaysStats.guessList
                .some(function (g) { return g.isCorrect });
            if (guessList.length === 6 && !victory) {
                // defeated; append correct solution
                guessList.push(todaysStats.correctAnswer);
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
        var statsButton = document.querySelector("header > div > div > div.justify-end > button");
        if (statsButton === null && statsButtonCount < 5) {
            statsButtonCount += 1;
            window.setTimeout(waitAndGiveStatsButton, 200);
        }
        setUpButton(statsButton);
    };
    window.setTimeout(waitAndGiveStatsButton, 200);
})();
