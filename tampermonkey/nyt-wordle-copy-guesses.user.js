// ==UserScript==
// @name         Wordle Copy Guesses - NYT
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/nyt
// @version      0.3
// @description  Provide a button to quickly copy Wordle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://www.nytimes.com/games/wordle/*
// @icon         https://www.nytimes.com/games/wordle/images/NYT-Wordle-Icon-32.png
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function getUserID() {
        // is the user logged in?
        var loginCookies = document.cookie
            .split("; ")
            .filter(function (ck) { return ck.startsWith("nyt-jkidd="); })
            .map(function (ck) { return ck.substring("nyt-jkidd=".length); });
        var userID = "ANON";
        for (var i = 0; i < loginCookies.length; i++) {
            if (loginCookies[i].length === 0) {
                continue;
            }
            var userIDValues = loginCookies[i]
                .split("&")
                .filter(function (v) { return v.startsWith("uid="); })
                .map(function (v) { return v.substring("uid=".length); });
            for (var j = 0; j < userIDValues.length; j++) {
                if (userIDValues[j].length === 0) {
                    continue;
                }
                if (!window.localStorage.hasOwnProperty("nyt-wordle-moogle/" + userIDValues[j])) {
                    continue;
                }
                userID = userIDValues[j];
            }
        }
        return userID;
    }

    function setUpButton(navBar) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.backgroundColor = "var(--key-bg)";
        copyButton.style.color = "var(--key-text-color)";
        navBar.insertBefore(copyButton, navBar.firstChild);

        copyButton.addEventListener("click", function () {
            var userID = getUserID();
            var wordleState = JSON.parse(window.localStorage["nyt-wordle-moogle/" + userID]);
            var guessList = wordleState.game.boardState
                .filter(function (s) { return s !== ''; })
                .map(function (s) { return s.toUpperCase(); });
            if (wordleState.game.status === "FAIL") {
                // defeated; the solution is no longer stored in localStorage...
                guessList.push("<insert correct solution>");
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
