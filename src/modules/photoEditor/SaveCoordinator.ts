/** Serialize only native project writes; remote work and consent never hold this queue. */
export class SaveCoordinator {
  private tail: Promise<unknown> = Promise.resolve();
  run<T>(write: () => Promise<T>): Promise<T> {
    const result = this.tail.then(write, write);
    this.tail = result.catch(() => undefined);
    return result;
  }
}
