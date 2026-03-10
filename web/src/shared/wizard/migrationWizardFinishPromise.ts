import api from '../api/api';
import { delay } from '../utils/delay';

// This waits until new core server starts and responds to session info
export const migrationWizardFinishPromise = async (): Promise<void> => {
  const timeoutMs = 120_000;
  const deadline = Date.now() + timeoutMs;

  const getRemainingMs = (): number => deadline - Date.now();

  function withRemainingTimeout<T>(promise: Promise<T>, operation: string): Promise<T> {
    const remainingMs = getRemainingMs();
    if (remainingMs <= 0) {
      return Promise.reject(
        new Error('Migration wizard finish timed out after 1 minute.'),
      );
    }

    return new Promise<T>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        reject(
          new Error(`Migration wizard finish timed out after 1 minute (${operation}).`),
        );
      }, remainingMs);

      promise
        .then((result) => {
          clearTimeout(timeoutId);
          resolve(result);
        })
        .catch((error) => {
          clearTimeout(timeoutId);
          reject(error);
        });
    });
  }

  await withRemainingTimeout(api.migration.finish(), 'finishing migration');

  while (true) {
    if (getRemainingMs() <= 0) {
      throw new Error('Migration wizard finish timed out after 1 minute.');
    }

    await delay(250);
    try {
      const sessionInfo = (
        await withRemainingTimeout(api.getSessionInfo(), 'checking session status')
      ).data;
      if (sessionInfo.active_wizard === null) {
        break;
      }
    } catch (error) {
      if (getRemainingMs() <= 0) {
        throw new Error('Migration wizard finish timed out after 1 minute.');
      }

      // Ignore transient connection failures while the new server starts.
      if (error instanceof Error && error.message.includes('timed out after 1 minute')) {
        throw error;
      }
    }
  }
};
