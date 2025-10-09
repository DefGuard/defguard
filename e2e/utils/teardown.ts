/* eslint-disable @typescript-eslint/no-unused-vars */
import { execSync } from 'child_process';
import { FullConfig } from 'playwright/test';

import { dockerCompose } from './docker';

// clear the db dump file
const teardownFunction = async (_: FullConfig) => {
  const command = `${dockerCompose} down`;
  execSync(command);
  console.log('Compose DOWN');
};

export default teardownFunction;
