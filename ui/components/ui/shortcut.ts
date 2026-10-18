export function shortcutKeys(shortcut: string, isMac: boolean): string[] {
  const modifiers: Record<string, { label: string; order: number }> = {
    commandorcontrol: { label: isMac ? '⌘' : 'Ctrl', order: isMac ? 3 : 0 },
    cmdorctrl: { label: isMac ? '⌘' : 'Ctrl', order: isMac ? 3 : 0 },
    command: { label: isMac ? '⌘' : 'Win', order: 3 },
    cmd: { label: isMac ? '⌘' : 'Win', order: 3 },
    super: { label: isMac ? '⌘' : 'Win', order: 3 },
    meta: { label: isMac ? '⌘' : 'Win', order: 3 },
    control: { label: isMac ? '⌃' : 'Ctrl', order: 0 },
    ctrl: { label: isMac ? '⌃' : 'Ctrl', order: 0 },
    alt: { label: isMac ? '⌥' : 'Alt', order: 1 },
    option: { label: isMac ? '⌥' : 'Alt', order: 1 },
    shift: { label: isMac ? '⇧' : 'Shift', order: 2 },
  };
  const keys: Record<string, string> = {
    space: '空格',
    enter: isMac ? '↵' : 'Enter',
    return: isMac ? '↵' : 'Enter',
    escape: 'Esc',
    esc: 'Esc',
    tab: isMac ? '⇥' : 'Tab',
    backspace: isMac ? '⌫' : 'Backspace',
    delete: isMac ? '⌦' : 'Delete',
    arrowup: '↑',
    arrowdown: '↓',
    arrowleft: '←',
    arrowright: '→',
  };
  return shortcut
    .split('+')
    .map((key) => key.trim())
    .filter(Boolean)
    .map((key) => {
      const normalized = key.toLowerCase();
      const modifier = Object.hasOwn(modifiers, normalized) ? modifiers[normalized] : undefined;
      if (modifier) return modifier;
      const label =
        (Object.hasOwn(keys, normalized) ? keys[normalized] : undefined) ??
        key.replace(/^Key([a-z])$/i, '$1').replace(/^Digit(\d)$/, '$1');
      return {
        label: label.length === 1 || /^f\d{1,2}$/i.test(label) ? label.toUpperCase() : label,
        order: 4,
      };
    })
    .sort((a, b) => a.order - b.order)
    .map((key) => key.label);
}
