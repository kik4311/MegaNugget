// Native <details> works without JS. Keep just one answer expanded at a time.
const answers = document.querySelectorAll(".faq-list details");
for (const answer of answers) {
  answer.addEventListener("toggle", () => {
    if (!answer.open) return;
    for (const other of answers) {
      if (other !== answer) other.open = false;
    }
  });
}
