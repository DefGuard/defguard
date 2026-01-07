import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

export const dockerCompose = `docker compose -f ${dockerFilePath}`;

export const restorePath = path.resolve(defguardPath, 'e2e', 'defguard_backup.dump');

const dbRestore = () => {
  const restore = `${dockerCompose} exec db pg_restore --clean -U defguard -d defguard /tmp/db.dump`;
  execSync(restore);
};

export const dockerRestart = () => {
  execSync(`${dockerCompose} stop core`);
  dbRestore();
  execSync(`${dockerCompose} start core`);
};
