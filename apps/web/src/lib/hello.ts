export function hello(name: string) {
  if (!name.trim()) return 'hello';
  return `hello ${name}`;
}
