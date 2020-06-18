import module from '../crate/Cargo.toml'

const tabSpaces = 4;

let {explore} = module;

console.log('loaded');

let input = document.getElementById('input') as HTMLTextAreaElement;
let output = document.getElementById('output');

function trigger() {
  if (!input || !output) {
    return;
  }
  explore(input.value, output);
}

input.addEventListener('keydown', (e) => {
  let keyCode = e.keyCode || e.which;

  if (keyCode == 9) {
    e.preventDefault();
    let start = input.selectionStart;
    let end = input.selectionEnd;
    let value = input.value;

    let before = value.substring(0, start);
    let after = value.substring(end);

    if (e.getModifierState('Shift')) {
      if (before.endsWith(' '.repeat(tabSpaces))) {
        input.value = before.substring(0, start - tabSpaces) + after;
        input.selectionStart = input.selectionEnd = start - tabSpaces;
      }
    } else {
      input.value = before + ' '.repeat(tabSpaces) + after;
      input.selectionStart = input.selectionEnd = start + tabSpaces;
    }
  }
})
input.addEventListener('input', trigger);

input.value = '(module)'
trigger();
