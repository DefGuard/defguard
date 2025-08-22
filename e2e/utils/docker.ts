import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

// Startups defguard stack with docker compose
export const dockerUp = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} up -d --wait`;
  execSync(command);
  execSync('docker exec -i defguard-db-1 sh -c "until pg_isready -h localhost -p 5432; do sleep 2; done"')
  execSync('docker exec -i defguard-db-1 pg_dump -U defguard -d defguard -Fc -f /tmp/defguard_backup.dump');
};

export const dockerDown = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} down`;
  if (dockerCheckContainers()) {
    execSync(command);
  }
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
    execSync('docker exec -i defguard-db-1 psql -U defguard -d defguard -c "DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public;"');
    execSync('docker exec -i defguard-db-1 pg_restore -U defguard -d defguard /tmp/defguard_backup.dump',{ stdio: 'inherit' })
  }
};

export const dockerStartup = () => {
  dockerDown();
  dockerUp();
};
