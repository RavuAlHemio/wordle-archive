// ==UserScript==
// @name         Wordle Copy Guesses - Worldle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/worldle
// @version      0.1
// @description  Provide a button to quickly copy Worldle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://worldle.teuteuf.fr
// @icon         https://www.google.com/s2/favicons?sz=64&domain=worldle.teuteuf.fr
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function leftPad(string, padChar, howMany) {
        string = "" + string;
        while (string.length < howMany) {
            string = padChar + string;
        }
        return string;
    }

    function setUpButton(footer) {
        var spacer = document.createTextNode(" ");
        footer.insertBefore(spacer, footer.firstChild);

        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        copyButton.style.border = "1px solid #fff";
        copyButton.style.padding = "0.2em";
        footer.insertBefore(copyButton, footer.firstChild);

        copyButton.addEventListener("click", function () {
            var currentDate = new Date();
            var currentDateIso =
                leftPad(currentDate.getFullYear(), "0", 4)
                + "-"
                + leftPad(currentDate.getMonth() + 1, "0", 2)
                + "-"
                + leftPad(currentDate.getDate(), "0", 2);

            var guesses = JSON.parse(window.localStorage.guesses);
            var todaysGuesses = guesses[currentDateIso];
            var guessList = todaysGuesses
                .map(function (g) { return g.name.toUpperCase(); });
            if (todaysGuesses.length === 6 && todaysGuesses[todaysGuesses.length - 1].distance > 0) {
                // defeated; hard to get the correct solution
                guessList.push("<correct solution here>");
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });
    }

    var waitAndGiveFooter;
    var footerCount = 0;
    waitAndGiveFooter = function () {
        var footer = null;
        var footers = document.getElementsByTagName("footer");
        if (footers.length > 0) {
            footer = footers[0];
        }
        if (footer === null && footerCount < 5) {
            footerCount += 1;
            window.setTimeout(waitAndGiveFooter, 200);
        }
        setUpButton(footer);
    };
    window.setTimeout(waitAndGiveFooter, 200);
})();
