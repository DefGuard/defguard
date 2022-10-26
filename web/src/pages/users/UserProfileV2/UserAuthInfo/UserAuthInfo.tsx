import './style.scss';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../shared/components/layout/Card/Card';
import Divider from '../../../../shared/components/layout/Divider/Divider';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';
import { UserAuthInfoRecovery } from './UserAuthInfoRecovery';

export const UserAuthInfo = () => {
  return (
    <section id="user-auth-info">
      <h2>Password and authentication</h2>
      <Card>
        <UserAuthInfoPassword />
        <UserAuthInfoMFA />
        <Divider />
        <UserAuthInfoRecovery />
      </Card>
    </section>
  );
};
