import { config } from 'dotenv';
import { existsSync } from 'fs';
import { resolve } from 'path';

const localPath = resolve(__dirname, '..', '.env.local');
const devPath = resolve(__dirname, '..', '.env.development');

export const loadEnv = () => {
  if (existsSync(localPath)) {
    config({ path: localPath });
    console.log(`Using env from ${localPath}`);
    return;
  }
  if (existsSync(devPath)) {
    config({ path: devPath });
    console.log(`Using env from ${devPath}`);
    return;
  }
  config();
};
