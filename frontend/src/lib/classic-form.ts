/**
 * Leaves the new UI through a classic form post: a hidden form with `fields` (names may repeat,
 * as `features[]` does) and the session's token in the parameter the Rust shell names in
 * `<meta name="csrf-param">`, submitted as the classic page's `button_to` would. For the steps
 * that must be full page loads, like an OAuth start that redirects to the provider.
 */
export function postClassicForm(
  action: string,
  fields: readonly (readonly [string, string])[] = [],
): void {
  const form = document.createElement("form");

  form.method = "post";
  form.action = action;
  form.hidden = true;

  const param = document.querySelector('meta[name="csrf-param"]')?.getAttribute("content");
  const token = document.querySelector('meta[name="csrf-token"]')?.getAttribute("content");
  const all = param && token ? [...fields, [param, token] as const] : fields;

  for (const [name, value] of all) {
    const input = document.createElement("input");

    input.type = "hidden";
    input.name = name;
    input.value = value;
    form.append(input);
  }

  document.body.append(form);
  form.submit();
}
