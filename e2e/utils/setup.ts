import { FullConfig } from 'playwright/test';

import { dockerCreateTemplate, dockerUp } from './docker';

const globalSetup = (_: FullConfig) => {
  dockerUp();
  dockerCreateTemplate();
};

export default globalSetup;
