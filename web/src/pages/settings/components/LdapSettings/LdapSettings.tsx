import './style.scss';

import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';

export const LdapSettings = () => {
  return (
    <div className="left">
      <section id="ldap-settings">
        <header>
          <h2></h2>
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text=""
            type="submit"
            loading={false}
          />
        </header>
        <form id="ldap-settings-form">
          <input type="submit" aria-hidden="true" className="hidden" />
        </form>
      </section>
    </div>
  );
};
