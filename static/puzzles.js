"use strict";
var WordleArchive;
(function (WordleArchive) {
    var Puzzles;
    (function (Puzzles) {
        Puzzles.puzzles = [];
        function performSetup() {
            var linkboxes = document.querySelectorAll(".puzzle-links");
            var _loop_1 = function (i) {
                var addMiddot = document.createTextNode(" \u00B7 ");
                linkboxes[i].appendChild(addMiddot);
                var copyLink = document.createElement("a");
                copyLink.href = "javascript:void(0)";
                copyLink.textContent = "copy";
                copyLink.addEventListener("click", function () {
                    navigator.clipboard.writeText(Puzzles.puzzles[i]).then(function () {
                        copyLink.textContent = "copied!";
                    }, function () {
                        copyLink.textContent = "failed! :-(";
                    });
                });
                linkboxes[i].appendChild(copyLink);
            };
            for (var i = 0; i < linkboxes.length; i++) {
                _loop_1(i);
            }
        }
        function setUp() {
            document.addEventListener("DOMContentLoaded", performSetup);
        }
        Puzzles.setUp = setUp;
    })(Puzzles = WordleArchive.Puzzles || (WordleArchive.Puzzles = {}));
})(WordleArchive || (WordleArchive = {}));
//# sourceMappingURL=puzzles.js.map