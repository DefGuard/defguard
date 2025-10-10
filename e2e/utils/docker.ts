import { execSync } from 'child_process';
import path from 'path';

import { dockerUp } from './setup';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

export const dockerCompose = `docker compose -f ${dockerFilePath}`;

export const restorePath = path.resolve(defguardPath, 'e2e', 'defguard_backup.dump');

export const dockerCheckContainers = (): boolean => {
  const command = `${dockerCompose} ps -q`;
  const containers = execSync(command).toString().trim();
  return Boolean(
    containers.length && containers.includes('core') && containers.includes('db'),
  );
};

const dbRestore = () => {
  const restore = `${dockerCompose} exec db pg_restore --clean -U defguard -d defguard /tmp/db.dump`;
  execSync(restore);
};

export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  } else {
    execSync(`${dockerCompose} stop core`);
    dbRestore();
    execSync(`${dockerCompose} start core`);
  }
};
