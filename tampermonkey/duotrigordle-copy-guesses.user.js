// ==UserScript==
// @name         Wordle Copy Guesses - Duotrigordle
// @namespace    http://ondrahosek.com/tampermonkey/wordle-copy-guesses/wordle32
// @version      0.1
// @description  Provide a button to quickly copy Duotrigordle guesses into the clipboard.
// @author       Ondřej Hošek <ondra.hosek@gmail.com>
// @match        https://duotrigordle.com/
// @match        https://duotrigordle.ro/
// @icon         https://www.google.com/s2/favicons?sz=64&domain=duotrigordle.com
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    function setUpButton(headerBar) {
        var copyButton = document.createElement("input");
        copyButton.type = "button";
        copyButton.value = "copy guesses";
        headerBar.insertBefore(copyButton, headerBar.lastChild);

        copyButton.addEventListener("click", function () {
            var state = JSON.parse(window.localStorage["duotrigordle-state"]);
            var guessList = state.guesses;

            var wordsDiv = document.querySelector("div.result > div.words");
            if (wordsDiv !== null) {
                var wordElements = wordsDiv.querySelectorAll("p");
                for (var i = 0; i < wordElements.length; i++) {
                    var word = wordElements.item(i).textContent;
                    if (guessList.indexOf(word) === -1) {
                        guessList.push(word);
                    }
                }
            } else {
                // no words to be found; add a hint for missed solutions
                guessList.push("<missed solutions here>");
            }
            var guesses = guessList.join("\n");

            navigator.clipboard.writeText(guesses).then(function () {
                copyButton.value = "copied!";
            }, function () {
                copyButton.value = "failed";
            });
        });

        // also enable double-clicking on a board
        var boards = document.querySelectorAll("div.boards > div.board");
        var i;
        const LINE_LENGTH = 5;
        for (i = 0; i < boards.length; i++) {
            var board = boards[i];
            board.addEventListener("dblclick", function () {
                var thisBoard = this;

                // collect the state of this board
                var cells = thisBoard.querySelectorAll("div.cell");
                var j, k;
                var lines = [];
                for (j = 0; j < cells.length/LINE_LENGTH; j++) {
                    var thisLine = "";
                    for (k = 0; k < LINE_LENGTH; k++) {
                        var cell = cells[LINE_LENGTH*j + k];
                        var cellLetter = cell.querySelector("span.letter");
                        if (cellLetter.textContent === "") {
                            continue;
                        }

                        if (cell.classList.contains("green")) {
                            thisLine += "c";
                        } else if (cell.classList.contains("yellow")) {
                            thisLine += "m";
                        } else {
                            thisLine += "w";
                        }
                    }
                    if (thisLine.length != LINE_LENGTH) {
                        break;
                    }
                    lines.push(thisLine);
                }
                var linesString = lines.join("\n");

                // copy to clipboard
                navigator.clipboard.writeText(linesString).then(function () {
                    alert("board colors copied");
                }, function () {
                    alert("board copy failed");
                });
            });
        }
    }

    var waitAndGiveHeaderBar;
    var headerBarCount = 0;
    waitAndGiveHeaderBar = function () {
        var headerBar = document.querySelector("#root .game .header .row-2");
        if (headerBar === null && headerBarCount < 5) {
            headerBarCount += 1;
            window.setTimeout(waitAndGiveHeaderBar, 200);
        }
        setUpButton(headerBar);
    };
    window.setTimeout(waitAndGiveHeaderBar, 200);
})();
