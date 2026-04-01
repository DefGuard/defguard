import api from '../api/api';
import { delay } from '../utils/delay';

export const migrationWizardFinishPromise = async (): Promise<void> => {
  await api.migration.finish();
  await delay(2000);
};
