// ==UserScript==
// @name         Wordle Copy Guesses - Heardle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/heardle
// @version      0.2.1
// @description  Provide a button to quickly copy Heardle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://www.spotify.com/heardle/*
// @match        https://proxy-nyc.hidemyass-freeproxy.com/proxy/*/aHR0cHM6Ly93d3cuc3BvdGlmeS5jb20vaGVhcmRsZS8
// @match        https://proxy-sea.hidemyass-freeproxy.com/proxy/*/aHR0cHM6Ly93d3cuc3BvdGlmeS5jb20vaGVhcmRsZS8
// @match        https://*.heardledecades.com/
// @match        https://reheardle.com/*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=www.heardle.app
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    var BREAK_SHARE_API = true;

    function setUpButton(statsButton) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.backgroundColor = "var(--color-mg)";
        copyButton.style.color = "var(--color-fg)";
        statsButton.parentElement.insertBefore(copyButton, statsButton);

        copyButton.addEventListener("click", function () {
            var stats;
            if (window.location.host === "reheardle.com") {
                var key;
                if (window.location.pathname === "/") {
                    key = "original";
                }
                else {
                    key = window.location.pathname.replaceAll("/", "");
                }
                stats = JSON.parse(window.localStorage[key]);
            }
            else {
                stats = JSON.parse(window.localStorage.userStats)
            }
            var todaysStats = stats[stats.length - 1];
            var guessList = todaysStats.guessList
                .map(function (g) { return (g.answer === undefined) ? "" : g.answer; });
            var victory = todaysStats.guessList
                .some(function (g) { return g.isCorrect });
            if (guessList.length === 6 && !victory) {
                // defeated; append correct solution
                if (todaysStats.correctAnswer !== undefined) {
                    guessList.push(todaysStats.correctAnswer);
                } else {
                    // missing from localStorage; extract from Spotify link
                    var titleRegex = /^Listen to (.+) by (.+) on Spotify$/;
                    var links = document.getElementsByTagName("a");
                    for (var i = 0; i < links.length; i++) {
                        var matches = titleRegex.exec(links[i].title);
                        if (matches === null) {
                            continue;
                        }
                        guessList.push(matches[2] + " - " + matches[1]);
                        break;
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

        if (BREAK_SHARE_API) {
            // also break the share API
            // (Windows and Chrome don't allow clipboard access through it)
            navigator.share = null;
        }
    }

    var waitAndGiveStatsButton;
    var statsButtonCount = 0;
    waitAndGiveStatsButton = function () {
        var statsButton = document.querySelector("div > div > div > div > h1 + button");
        if (statsButton === null) {
            statsButton = document.querySelector("div > div > div:last-child > button");
        }
        if (statsButton === null && statsButtonCount < 5) {
            statsButtonCount += 1;
            window.setTimeout(waitAndGiveStatsButton, 200);
        }
        setUpButton(statsButton);
    };
    window.setTimeout(waitAndGiveStatsButton, 200);
})();
