"use strict";
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var WordleArchive;
(function (WordleArchive) {
    var Wordle32Spoiler;
    (function (Wordle32Spoiler) {
        var CodePoints = /** @class */ (function () {
            function CodePoints(codePoints) {
                this.codePoints = codePoints;
            }
            CodePoints.fromString = function (s) {
                var codePoints = [];
                var expectant = null;
                for (var i = 0; i < s.length; i++) {
                    var c = s.charCodeAt(i);
                    if (expectant !== null) {
                        // last time we read a leading surrogate
                        if (c < 0xDC00 || c > 0xDFFF) {
                            // leading surrogate followed by something that is not a trailing surrogate
                            return null;
                        }
                        codePoints.push(expectant + (c - 0xDC00));
                        expectant = null;
                    }
                    else if (c >= 0xD800 && c <= 0xDBFF) {
                        // leading surrogate
                        expectant = ((c - 0xD800) << 10) + 0x10000;
                    }
                    else if (c >= 0xDC00 && c <= 0xDFFF) {
                        // trailing surrogate without a leading surrogate
                        // (otherwise expectant would not be null)
                        return null;
                    }
                    else {
                        // BMP character
                        codePoints.push(c);
                    }
                }
                return new CodePoints(codePoints);
            };
            CodePoints.prototype.toString = function () {
                var e_1, _a;
                var buf = [];
                try {
                    for (var _b = __values(this.codePoints), _c = _b.next(); !_c.done; _c = _b.next()) {
                        var c = _c.value;
                        if (c >= 0x10000) {
                            var cRest = c - 0x10000;
                            var leading = ((cRest >> 10) & 0x3FF) + 0xD800;
                            var trailing = ((cRest >> 0) & 0x3FF) + 0xDC00;
                            buf.push(String.fromCharCode(leading, trailing));
                        }
                        else {
                            // BMP character
                            buf.push(String.fromCharCode(c));
                        }
                    }
                }
                catch (e_1_1) { e_1 = { error: e_1_1 }; }
                finally {
                    try {
                        if (_c && !_c.done && (_a = _b["return"])) _a.call(_b);
                    }
                    finally { if (e_1) throw e_1.error; }
                }
                return buf.join("");
            };
            CodePoints.prototype.toArray = function () {
                return this.codePoints.map(function (cp) { return cp; });
            };
            return CodePoints;
        }());
        function rate(guess, solution) {
            var guessCP = guess.toArray();
            var solCP = solution.toArray();
            if (guessCP.length !== solCP.length) {
                return null;
            }
            var rating = [];
            for (var i = 0; i < guessCP.length; i++) {
                rating.push('W');
            }
            // find correct letters
            for (var i = 0; i < guessCP.length; i++) {
                if (guessCP[i] === solCP[i]) {
                    guessCP[i] = -1;
                    solCP[i] = -1;
                    rating[i] = 'C';
                }
            }
            // find misplaced letters
            for (var g = 0; g < guessCP.length; g++) {
                if (guessCP[g] === -1) {
                    continue;
                }
                for (var s = 0; s < solCP.length; s++) {
                    if (solCP[s] === -1) {
                        continue;
                    }
                    if (guessCP[g] === solCP[s]) {
                        guessCP[g] = -1;
                        solCP[s] = -1;
                        // rating is relative to guess
                        rating[g] = 'M';
                    }
                }
            }
            return rating;
        }
        var Spoiler = /** @class */ (function () {
            function Spoiler(puzzle_id, sub_puzzle_index) {
                this.puzzle_id = puzzle_id;
                this.sub_puzzle_index = sub_puzzle_index;
                this.solutions = [];
            }
            return Spoiler;
        }());
        var Solution = /** @class */ (function () {
            function Solution() {
                this.guesses = [];
            }
            return Solution;
        }());
        var Guess = /** @class */ (function () {
            function Guess(ratings) {
                this.ratings = ratings;
            }
            return Guess;
        }());
        var knownSpoilers = [];
        function register(puzzle_id, sub_puzzle_index) {
            var _a;
            var puzzle = document.querySelector(".puzzle-id-".concat(puzzle_id, " .sub-puzzle-index-").concat(sub_puzzle_index));
            if (puzzle === null) {
                return;
            }
            var spoiler = new Spoiler(puzzle_id, sub_puzzle_index);
            // preprocess guesses
            var guessRows = puzzle.querySelectorAll(".all-guess-row");
            var guessWords = [];
            for (var g = 0; g < guessRows.length; g++) {
                var guessRow = guessRows[g];
                var letters = [];
                var letterBoxes = guessRow.querySelectorAll(".solution-box");
                for (var l = 0; l < letterBoxes.length; l++) {
                    letters.push((_a = letterBoxes[l].textContent) !== null && _a !== void 0 ? _a : "");
                }
                var guessWord = CodePoints.fromString(letters.join(""));
                if (guessWord !== null) {
                    guessWords.push(guessWord);
                }
            }
            var solutionBoxes = puzzle.querySelectorAll(".solution-row .solution-box");
            // calculate ratings
            for (var s = 0; s < solutionBoxes.length; s++) {
                var solutionBox = solutionBoxes[s];
                var solutionString = solutionBox.textContent;
                if (solutionString === null) {
                    continue;
                }
                var solutionWord = CodePoints.fromString(solutionString);
                if (solutionWord === null) {
                    continue;
                }
                var solution = new Solution();
                for (var g = 0; g < guessWords.length; g++) {
                    var rating = rate(guessWords[g], solutionWord);
                    if (rating === null) {
                        continue;
                    }
                    solution.guesses.push(new Guess(rating));
                }
                spoiler.solutions.push(solution);
            }
            knownSpoilers.push(spoiler);
            var _loop_1 = function (s) {
                var solutionBox = solutionBoxes[s];
                solutionBox.addEventListener("mouseover", function () { return colorBoxes(puzzle_id, sub_puzzle_index, s); });
                solutionBox.addEventListener("mouseout", function () { return uncolorBoxes(puzzle_id, sub_puzzle_index); });
            };
            // link up events
            for (var s = 0; s < solutionBoxes.length; s++) {
                _loop_1(s);
            }
        }
        Wordle32Spoiler.register = register;
        function colorBoxes(puzzle_id, sub_puzzle_index, solution_index) {
            var e_2, _a;
            // find the spoiler
            var spoiler = null;
            try {
                for (var knownSpoilers_1 = __values(knownSpoilers), knownSpoilers_1_1 = knownSpoilers_1.next(); !knownSpoilers_1_1.done; knownSpoilers_1_1 = knownSpoilers_1.next()) {
                    var knownSpoiler = knownSpoilers_1_1.value;
                    if (knownSpoiler.puzzle_id === puzzle_id && knownSpoiler.sub_puzzle_index == sub_puzzle_index) {
                        spoiler = knownSpoiler;
                        break;
                    }
                }
            }
            catch (e_2_1) { e_2 = { error: e_2_1 }; }
            finally {
                try {
                    if (knownSpoilers_1_1 && !knownSpoilers_1_1.done && (_a = knownSpoilers_1["return"])) _a.call(knownSpoilers_1);
                }
                finally { if (e_2) throw e_2.error; }
            }
            if (spoiler === null) {
                return;
            }
            // pick out the solution
            var solution = spoiler.solutions[solution_index];
            // find the boxes
            var puzzle = document.querySelector(".puzzle-id-".concat(puzzle_id, " .sub-puzzle-index-").concat(sub_puzzle_index));
            if (puzzle === null) {
                return;
            }
            var guessRows = puzzle.querySelectorAll(".all-guess-row");
            for (var g = 0; g < solution.guesses.length; g++) {
                var guess = solution.guesses[g];
                var guessRow = guessRows[g];
                var guessBoxes = guessRow.querySelectorAll(".solution-box");
                var allRatingsAreC = true;
                for (var l = 0; l < guess.ratings.length; l++) {
                    var rating = guess.ratings[l];
                    var guessBox = guessBoxes[l];
                    if (rating === 'C') {
                        guessBox.classList.add("rating-correct");
                    }
                    else if (rating === 'M') {
                        guessBox.classList.add("rating-misplaced");
                        allRatingsAreC = false;
                    }
                    else if (rating === 'W') {
                        guessBox.classList.add("rating-wrong");
                        allRatingsAreC = false;
                    }
                }
                if (allRatingsAreC) {
                    // correct guess -- stop coloring here
                    break;
                }
            }
        }
        function uncolorBoxes(puzzle_id, sub_puzzle_index) {
            var guessBoxes = document.querySelectorAll(".puzzle-id-".concat(puzzle_id, " .sub-puzzle-index-").concat(sub_puzzle_index, " .all-guess-row .solution-box"));
            for (var i = 0; i < guessBoxes.length; i++) {
                guessBoxes[i].classList.remove("rating-correct");
                guessBoxes[i].classList.remove("rating-misplaced");
                guessBoxes[i].classList.remove("rating-wrong");
            }
        }
    })(Wordle32Spoiler = WordleArchive.Wordle32Spoiler || (WordleArchive.Wordle32Spoiler = {}));
})(WordleArchive || (WordleArchive = {}));
//# sourceMappingURL=wordle32-spoiler.js.map