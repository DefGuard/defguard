import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { ChangeEvent, useEffect, useState } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgIconCheckmark from '../../../../shared/components/svg/IconCheckmark';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { TextareaAutoResizable } from '../../../../shared/defguard-ui/components/Layout/TextareaAutoResizable/TextareaAutoResizable';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const EnrollmentWelcomeMessage = () => {
  const {
    settings: { editSettings },
  } = useApi();
  const settings = useEnrollmentStore((state) => state.settings);
  const [message, setMessage] = useState(settings?.enrollment_welcome_message ?? '');
  const { LL } = useI18nContext();
  const componentLL = LL.enrollmentPage.settings.welcomeMessage;
  const queryClient = useQueryClient();
  const toaster = useToaster();

  const { isPending: isLoading, mutate } = useMutation({
    mutationFn: editSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_SETTINGS],
      });
      toaster.success(LL.enrollmentPage.messages.edit.success());
    },
    onError: (e) => {
      toaster.error(LL.enrollmentPage.messages.edit.error());
      console.error(e);
    },
  });

  useEffect(() => {
    if (settings) {
      setMessage(settings?.enrollment_welcome_message);
    }
  }, [settings]);

  return (
    <div id="enrollment-welcome-message">
      <header>
        <h3>{componentLL.title()}</h3>
      </header>
      <MessageBox type={MessageBoxType.INFO} message={componentLL.messageBox()} />
      <Card shaded hideMobile>
        <div className="controls">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            icon={<SvgIconCheckmark />}
            text={LL.enrollmentPage.controls.save()}
            loading={isUndefined(settings) || isLoading}
            onClick={() => {
              if (!isLoading && settings) {
                mutate({ ...settings, enrollment_welcome_message: message });
              }
            }}
          />
        </div>
        <div className="text-wrapper">
          <TextareaAutoResizable
            value={message}
            onChange={(ev: ChangeEvent<HTMLTextAreaElement>) =>
              setMessage(ev.target.value)
            }
            disabled={isUndefined(settings) || isLoading}
          />
        </div>
      </Card>
    </div>
  );
};
