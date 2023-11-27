import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

// Startups defguard stack with docker compose
export const dockerUp = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} up -d`;
  execSync(command);
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
  dockerDown();
  dockerUp();
};

export const dockerStartup = () => {
  dockerDown();
  dockerUp();
};
