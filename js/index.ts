import module from '../crate/Cargo.toml'

const tabSpaces = 4;

let {input_text} = module;

console.log('loaded');

let text = document.getElementById('text') as HTMLTextAreaElement;
let binary = document.getElementById('binary');
let explain = document.getElementById('explain');

function trigger() {
  if (!text || !binary || !explain) {
    return;
  }
  input_text(text.value, binary, explain);
}

text.addEventListener('keydown', (e) => {
  let keyCode = e.keyCode || e.which;

  if (keyCode == 9) {
    e.preventDefault();
    let start = text.selectionStart;
    let end = text.selectionEnd;
    let value = text.value;

    let before = value.substring(0, start);
    let after = value.substring(end);

    if (e.getModifierState('Shift')) {
      if (before.endsWith(' '.repeat(tabSpaces))) {
        text.value = before.substring(0, start - tabSpaces) + after;
        text.selectionStart = text.selectionEnd = start - tabSpaces;
      }
    } else {
      text.value = before + ' '.repeat(tabSpaces) + after;
      text.selectionStart = text.selectionEnd = start + tabSpaces;
    }
  }
})
text.addEventListener('input', trigger);

text.value = '(module)'
trigger();
