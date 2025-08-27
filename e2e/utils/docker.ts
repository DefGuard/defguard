import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');
const dockerCompose = `docker compose -f ${dockerFilePath}`

// Start Defguard stack with docker compose.
export const dockerUp = () => {
  const command = `${dockerCompose} up --wait`;
  execSync(command);
  // NOTE: After waiting, sleep for 3 seconds to let Defguard Core apply migrations.
  const wait_for_db = `${dockerCompose} exec db sh -c 'until pg_isready; do sleep 1; done; sleep 3'`;
  execSync(wait_for_db);
  const create_snapshot = `${dockerCompose} exec db pg_dump -U defguard -Fc -f /tmp/defguard_backup.dump defguard`;
  execSync(create_snapshot);
};

export const dockerCheckContainers = (): boolean => {
  const command = `${dockerCompose} ps -q`;
  const containers = execSync(command).toString().trim();
  return Boolean(containers.length);
};

export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  } else {
    const restore = `${dockerCompose} exec db pg_restore --clean -U defguard -d defguard /tmp/defguard_backup.dump`;
    execSync(restore);
    const restart = `${dockerCompose} restart db`;
    execSync(restart);
    const wait_for_db = `${dockerCompose} exec db sh -c 'until pg_isready; do sleep 1; done'`;
    execSync(wait_for_db);
  }
};
