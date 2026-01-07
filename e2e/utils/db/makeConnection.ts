import { Client } from 'pg';

import { dbCred } from '../../config';

const dbConfig = {
  user: dbCred.username,
  password: dbCred.password,
  database: dbCred.database,
  host: dbCred.host,
  port: dbCred.port,
};

export const makeConnection = async () => {
  const client = new Client(dbConfig);
  await client.connect();
  return client;
};
