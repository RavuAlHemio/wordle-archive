module WordleArchive.Wordle32Spoiler {
    class CodePoints {
        public codePoints: number[];

        public constructor(codePoints: number[]) {
            this.codePoints = codePoints;
        }

        public static fromString(s: string): CodePoints|null {
            const codePoints: number[] = [];
            let expectant: number|null = null;
            for (let i = 0; i < s.length; i++) {
                const c = s.charCodeAt(i);
                if (expectant !== null) {
                    // last time we read a leading surrogate
                    if (c < 0xDC00 || c > 0xDFFF) {
                        // leading surrogate followed by something that is not a trailing surrogate
                        return null;
                    }
                    codePoints.push(expectant + (c - 0xDC00));
                    expectant = null;
                } else if (c >= 0xD800 && c <= 0xDBFF) {
                    // leading surrogate
                    expectant = ((c - 0xD800) << 10) + 0x10000;
                } else if (c >= 0xDC00 && c <= 0xDFFF) {
                    // trailing surrogate without a leading surrogate
                    // (otherwise expectant would not be null)
                    return null;
                } else {
                    // BMP character
                    codePoints.push(c);
                }
            }
            return new CodePoints(codePoints);
        }

        public toString(): string {
            const buf: string[] = [];
            for (const c of this.codePoints) {
                if (c >= 0x10000) {
                    const cRest = c - 0x10000;
                    const leading = ((cRest >> 10) & 0x3FF) + 0xD800;
                    const trailing = ((cRest >> 0) & 0x3FF) + 0xDC00;
                    buf.push(String.fromCharCode(leading, trailing));
                } else {
                    // BMP character
                    buf.push(String.fromCharCode(c));
                }
            }
            return buf.join("");
        }

        public toArray(): number[] {
            return this.codePoints.map(cp => cp);
        }
    }

    type RatingLetter = 'C'|'M'|'W';

    function rate(guess: CodePoints, solution: CodePoints): RatingLetter[]|null {
        const guessCP = guess.toArray();
        const solCP = solution.toArray();

        if (guessCP.length !== solCP.length) {
            return null;
        }

        const rating: RatingLetter[] = [];
        for (let i = 0; i < guessCP.length; i++) {
            rating.push('W');
        }

        // find correct letters
        for (let i = 0; i < guessCP.length; i++) {
            if (guessCP[i] === solCP[i]) {
                guessCP[i] = -1;
                solCP[i] = -1;
                rating[i] = 'C';
            }
        }

        // find misplaced letters
        for (let g = 0; g < guessCP.length; g++) {
            if (guessCP[g] === -1) {
                continue;
            }

            for (let s = 0; s < solCP.length; s++) {
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

    class Spoiler {
        public puzzle_id: number;
        public sub_puzzle_index: number;
        public solutions: Solution[];

        constructor(puzzle_id: number, sub_puzzle_index: number) {
            this.puzzle_id = puzzle_id;
            this.sub_puzzle_index = sub_puzzle_index;
            this.solutions = [];
        }
    }

    class Solution {
        public guesses: Guess[];

        constructor() {
            this.guesses = [];
        }
    }

    class Guess {
        public ratings: RatingLetter[];

        constructor(ratings: RatingLetter[]) {
            this.ratings = ratings;
        }
    }

    let knownSpoilers: Spoiler[] = [];

    export function register(puzzle_id: number, sub_puzzle_index: number) {
        const puzzle = <HTMLDivElement|null>document.querySelector(`.puzzle-id-${puzzle_id} .sub-puzzle-index-${sub_puzzle_index}`);
        if (puzzle === null) {
            return;
        }

        const spoiler = new Spoiler(puzzle_id, sub_puzzle_index);

        // preprocess guesses
        const guessRows = <NodeListOf<HTMLDivElement>>puzzle.querySelectorAll(".all-guess-row");
        const guessWords: CodePoints[] = [];
        for (let g = 0; g < guessRows.length; g++) {
            const guessRow = guessRows[g];
            const letters: string[] = [];
            const letterBoxes = <NodeListOf<HTMLDivElement>>guessRow.querySelectorAll(".solution-box");
            for (let l = 0; l < letterBoxes.length; l++) {
                letters.push(letterBoxes[l].textContent ?? "");
            }
            const guessWord = CodePoints.fromString(letters.join(""));
            if (guessWord !== null) {
                guessWords.push(guessWord);
            }
        }

        const solutionBoxes = <NodeListOf<HTMLDivElement>>puzzle.querySelectorAll(".solution-row .solution-box");

        // calculate ratings
        for (let s = 0; s < solutionBoxes.length; s++) {
            const solutionBox = solutionBoxes[s];

            const solutionString = solutionBox.textContent;
            if (solutionString === null) {
                continue;
            }

            const solutionWord = CodePoints.fromString(solutionString);
            if (solutionWord === null) {
                continue;
            }

            const solution = new Solution();
            for (let g = 0; g < guessWords.length; g++) {
                const rating = rate(guessWords[g], solutionWord);
                if (rating === null) {
                    continue;
                }

                solution.guesses.push(new Guess(rating));
            }

            spoiler.solutions.push(solution);
        }

        knownSpoilers.push(spoiler);

        // link up events
        for (let s = 0; s < solutionBoxes.length; s++) {
            const solutionBox = solutionBoxes[s];
            solutionBox.addEventListener("mouseover", () => colorBoxes(puzzle_id, sub_puzzle_index, s));
            solutionBox.addEventListener("mouseout", () => uncolorBoxes(puzzle_id, sub_puzzle_index));
        }
    }

    function colorBoxes(puzzle_id: number, sub_puzzle_index: number, solution_index: number) {
        // find the spoiler
        let spoiler = null;
        for (let knownSpoiler of knownSpoilers) {
            if (knownSpoiler.puzzle_id === puzzle_id && knownSpoiler.sub_puzzle_index == sub_puzzle_index) {
                spoiler = knownSpoiler;
                break;
            }
        }
        if (spoiler === null) {
            return;
        }

        // pick out the solution
        const solution = spoiler.solutions[solution_index];

        // find the boxes
        const puzzle = document.querySelector(`.puzzle-id-${puzzle_id} .sub-puzzle-index-${sub_puzzle_index}`);
        if (puzzle === null) {
            return;
        }
        const guessRows = puzzle.querySelectorAll(".all-guess-row");
        for (let g = 0; g < solution.guesses.length; g++) {
            const guess = solution.guesses[g];
            const guessRow = guessRows[g];

            const guessBoxes = guessRow.querySelectorAll(".solution-box");

            let allRatingsAreC = true;
            for (let l = 0; l < guess.ratings.length; l++) {
                const rating = guess.ratings[l];
                const guessBox = guessBoxes[l];

                if (rating === 'C') {
                    guessBox.classList.add("rating-correct");
                } else if (rating === 'M') {
                    guessBox.classList.add("rating-misplaced");
                    allRatingsAreC = false;
                } else if (rating === 'W') {
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

    function uncolorBoxes(puzzle_id: number, sub_puzzle_index: number) {
        const guessBoxes = document.querySelectorAll(`.puzzle-id-${puzzle_id} .sub-puzzle-index-${sub_puzzle_index} .all-guess-row .solution-box`);
        for (let i = 0; i < guessBoxes.length; i++) {
            guessBoxes[i].classList.remove("rating-correct");
            guessBoxes[i].classList.remove("rating-misplaced");
            guessBoxes[i].classList.remove("rating-wrong");
        }
    }
}
