import './style.scss';

import React from 'react';

import UserForm from './UserForm';
import UsersTable from './UsersTable';

const Users: React.FC = () => (
  <div className="container-basic users">
    <UserForm />
    <UsersTable />
  </div>
);
export default Users;
