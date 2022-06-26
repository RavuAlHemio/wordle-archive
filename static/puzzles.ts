module WordleArchive.Puzzles {
    export let puzzles: string[] = [];

    function performSetup() {
        let linkboxes = document.querySelectorAll(".puzzle-links");
        for (let i = 0; i < linkboxes.length; i++) {
            let addMiddot = document.createTextNode(" \u00B7 ");
            linkboxes[i].appendChild(addMiddot);

            let copyLink = document.createElement("a");
            copyLink.href = "javascript:void(0)";
            copyLink.textContent = "copy";
            copyLink.addEventListener("click", () => {
                navigator.clipboard.writeText(puzzles[i]).then(function () {
                    copyLink.textContent = "copied!";
                }, function () {
                    copyLink.textContent = "failed! :-(";
                });
            });
            linkboxes[i].appendChild(copyLink);
        }
    }

    export function setUp() {
        document.addEventListener("DOMContentLoaded", performSetup);
    }
}
