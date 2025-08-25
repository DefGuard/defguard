import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

// Startups defguard stack with docker compose
export const dockerUp = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} up -d --wait`;
  execSync(command);
  const wait_for_db = `docker compose exec db sh -c "until pg_isready -h localhost -p 5432; do sleep 2; done"`
  execSync(wait_for_db); // wait for database
  const create_snapshot = `docker compose exec db pg_dump -U defguard -d defguard -Fc -f /tmp/defguard_backup.dump`
  execSync(create_snapshot); // create snapshot of db


};
export const dockerCheckContainers = (): boolean => {
  const command = `docker ps -q`;
  const containers = execSync(command).toString().trim();
  return Boolean(containers.length);
};

export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  } else {

    const restore = `docker compose exec db pg_restore --clean -U defguard -d defguard /tmp/defguard_backup.dump`;
    execSync(restore);
    const restart = `docker compose -f ${dockerFilePath.toString()} restart db`;
    execSync(restart);
  }
};