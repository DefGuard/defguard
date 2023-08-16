import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect, useState } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgIconCheckmark from '../../../../shared/components/svg/IconCheckmark';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CheckBox } from '../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const EnrollmentEmail = () => {
  const {
    settings: { editSettings },
  } = useApi();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const [duplicateMessage, setDuplicateMessage] = useState(false);
  const [email, setEmail] = useState('');
  const componentLL = LL.enrollmentPage.settings.welcomeEmail;
  const settings = useEnrollmentStore((state) => state.settings);
  const toaster = useToaster();

  const { isLoading, mutate } = useMutation({
    mutationFn: editSettings,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.enrollmentPage.messages.edit.success());
    },
    onError: (e) => {
      toaster.error(LL.enrollmentPage.messages.edit.error());
      console.error(e);
    },
  });

  const handleSave = () => {
    if (!isLoading && settings) {
      mutate({
        ...settings,
        enrollment_use_welcome_message_as_email: duplicateMessage,
        enrollment_welcome_email: email,
      });
    }
  };

  useEffect(() => {
    if (settings) {
      setDuplicateMessage(settings.enrollment_use_welcome_message_as_email);
      setEmail(settings.enrollment_welcome_email);
    }
    //eslint-disable-next-line
  }, []);

  return (
    <div id="enrollment-email">
      <header>
        <h3>{componentLL.title()}</h3>
      </header>
      <MessageBox type={MessageBoxType.INFO} message={componentLL.messageBox()} />
      <Card shaded hideMobile>
        <div className="controls">
          <div className="checkbox-wrap">
            <CheckBox
              value={duplicateMessage}
              onChange={() => setDuplicateMessage((state) => !state)}
              disabled={isLoading}
            />
            <span onClick={() => setDuplicateMessage((state) => !state)}>
              {componentLL.controls.duplicateWelcome()}
            </span>
          </div>
          <Button
            text={LL.enrollmentPage.controls.save()}
            styleVariant={ButtonStyleVariant.SAVE}
            size={ButtonSize.SMALL}
            icon={<SvgIconCheckmark />}
            onClick={() => handleSave()}
            loading={isLoading}
            disabled={isUndefined(settings)}
          />
        </div>
        <textarea
          value={email}
          onChange={(ev) => setEmail(ev.target.value)}
          disabled={isLoading || isUndefined(settings)}
        />
      </Card>
    </div>
  );
};
