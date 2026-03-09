import api from '../api/api';
import { delay } from '../utils/delay';

// This waits until new core server starts up and respond with
export const migrationWizardFinishPromise = async (): Promise<void> => {
  await api.migration.finish();
  while (true) {
    await delay(250);
    try {
      const sessionInfo = (await api.getSessionInfo()).data;
      if (sessionInfo.active_wizard === null) {
        break;
      }
    } catch (_) {}
  }
};
