export default function debounce<TThis, TArgs extends unknown[]>(
  func: (this: TThis, ...args: TArgs) => void,
  wait: number,
): (this: TThis, ...args: TArgs) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  return function (this: TThis, ...args: TArgs) {
    if (timeout !== null) {
      clearTimeout(timeout);
    }
    timeout = setTimeout(() => func.apply(this, args), wait);
  };
}
