import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('/e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

// Startups defguard stack with docker compose
export const dockerUp = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} up -d core db gateway`;
  execSync(command);
};

export const dockerDown = () => {
  const command = `docker compose -f ${dockerFilePath.toString()} down`;
  execSync(command);
};

export const dockerRestart = () => {
  dockerDown();
  dockerUp();
};
