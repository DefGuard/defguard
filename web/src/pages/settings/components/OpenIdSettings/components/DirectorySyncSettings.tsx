import './style.scss';

import parse from 'html-react-parser';
import { useMemo, useState } from 'react';
import { useFormContext, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import SvgIconDownload from '../../../../../shared/defguard-ui/components/svg/IconDownload';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { titleCase } from '../../../../../shared/utils/titleCase';

const SUPPORTED_SYNC_PROVIDERS = ['Google'];

export const DirsyncSettings = ({ isLoading }: { isLoading: boolean }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const [googleServiceAccountFileName, setGoogleServiceAccountFileName] = useState<
    string | null
  >(null);
  const {
    settings: { testDirsync },
  } = useApi();
  const { control, setValue } = useFormContext();
  const toaster = useToaster();

  const userBehaviorOptions = useMemo(
    () => [
      {
        value: 'keep',
        label: localLL.form.selects.behavior.keep(),
        key: 1,
      },
      {
        value: 'disable',
        label: localLL.form.selects.behavior.disable(),
        key: 2,
      },
      {
        value: 'delete',
        label: localLL.form.selects.behavior.delete(),
        key: 3,
      },
    ],
    [localLL.form.selects.behavior],
  );

  const syncTarget = useMemo(
    () => [
      {
        value: 'all',
        label: localLL.form.selects.synchronize.all(),
        key: 1,
      },
      {
        value: 'users',
        label: localLL.form.selects.synchronize.users(),
        key: 2,
      },
      {
        value: 'groups',
        label: localLL.form.selects.synchronize.groups(),
        key: 3,
      },
    ],
    [localLL.form.selects.synchronize],
  );

  const providerName = useWatch({ control, name: 'name' }) as string;
  const dirsyncEnabled: boolean = useWatch({
    control,
    name: 'directory_sync_enabled',
  }) as boolean;
  const showDirsync = SUPPORTED_SYNC_PROVIDERS.includes(providerName ?? '');

  return (
    <section id="dirsync-settings">
      <header id="dirsync-header">
        <h2>{localLL.form.directory_sync_settings.title()}</h2>
        <Helper>{localLL.form.directory_sync_settings.helper()}</Helper>
      </header>
      <div id="directory-sync-settings">
        {showDirsync ? (
          providerName === 'Google' ? (
            <>
              <div id="enable-dir-sync">
                {/* FIXME: Really buggy when using the controller, investigate why */}
                <LabeledCheckbox
                  disabled={isLoading || !showDirsync}
                  label={localLL.form.labels.enable_directory_sync.label()}
                  value={dirsyncEnabled}
                  onChange={(val) => setValue('directory_sync_enabled', val)}
                  // controller={{ control, name: 'directory_sync_enabled' }}
                />
              </div>
              <FormSelect
                controller={{ control, name: 'directory_sync_target' }}
                options={syncTarget}
                label={localLL.form.labels.sync_target.label()}
                renderSelected={(val) => ({
                  key: val,
                  displayValue: titleCase(val),
                })}
                labelExtras={
                  <Helper>{parse(localLL.form.labels.sync_target.helper())}</Helper>
                }
                disabled={isLoading}
              />
              <FormInput
                controller={{ control, name: 'directory_sync_interval' }}
                type="number"
                name="directory_sync_interval"
                label={localLL.form.labels.sync_interval.label()}
                required
                labelExtras={
                  <Helper>{parse(localLL.form.labels.sync_interval.helper())}</Helper>
                }
                disabled={isLoading}
              />
              <FormSelect
                controller={{ control, name: 'directory_sync_user_behavior' }}
                options={userBehaviorOptions}
                label={localLL.form.labels.user_behavior.label()}
                renderSelected={(val) => ({
                  key: val,
                  displayValue: titleCase(val),
                })}
                labelExtras={
                  <Helper>{parse(localLL.form.labels.user_behavior.helper())}</Helper>
                }
                disabled={isLoading}
              />
              <FormSelect
                controller={{ control, name: 'directory_sync_admin_behavior' }}
                options={userBehaviorOptions}
                label={localLL.form.labels.admin_behavior.label()}
                renderSelected={(val) => ({
                  key: val,
                  displayValue: titleCase(val),
                })}
                labelExtras={
                  <Helper>{parse(localLL.form.labels.admin_behavior.helper())}</Helper>
                }
                disabled={isLoading}
              />
              <FormInput
                controller={{ control, name: 'admin_email' }}
                label={localLL.form.labels.admin_email.label()}
                disabled={isLoading}
                labelExtras={
                  <Helper>{parse(localLL.form.labels.admin_email.helper())}</Helper>
                }
                required={dirsyncEnabled}
              />
              <FormInput
                controller={{ control, name: 'google_service_account_email' }}
                type="text"
                name="google_service_account_email"
                readOnly
                label={localLL.form.labels.service_account_used.label()}
                labelExtras={
                  <Helper>
                    {parse(localLL.form.labels.service_account_used.helper())}
                  </Helper>
                }
                disabled={isLoading}
                required={dirsyncEnabled}
              />
              <div className="input">
                <div className="top">
                  <label className="input-label">
                    {localLL.form.labels.service_account_key_file.label()}:
                  </label>
                  <Helper>{localLL.form.labels.service_account_key_file.helper()}</Helper>
                </div>
                <div className={'file-upload-container'}>
                  <input
                    className={'file-upload'}
                    type="file"
                    accept=".json"
                    onChange={(e) => {
                      const file = e.target.files?.[0];
                      if (file) {
                        const reader = new FileReader();
                        reader.onload = (e) => {
                          if (e?.target?.result) {
                            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
                            const key = JSON.parse(e.target?.result as string);
                            // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
                            setValue('google_service_account_key', key.private_key);
                            // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
                            setValue('google_service_account_email', key.client_email);
                            setGoogleServiceAccountFileName(file.name);
                          }
                        };
                        reader.readAsText(file);
                      }
                    }}
                    disabled={isLoading}
                  />
                  <div className="upload-label">
                    <SvgIconDownload />{' '}
                    <p>
                      {googleServiceAccountFileName
                        ? `${localLL.form.labels.service_account_key_file.uploaded()}: ${googleServiceAccountFileName}`
                        : localLL.form.labels.service_account_key_file.uploadPrompt()}
                    </p>
                  </div>
                </div>
              </div>
              <div className="test-connection">
                <Button
                  onClick={() => {
                    void testDirsync().then((res) => {
                      if (res.success) {
                        toaster.success(
                          localLL.form.directory_sync_settings.connectionTest.success(),
                        );
                      } else {
                        toaster.error(
                          `${localLL.form.directory_sync_settings.connectionTest.error()} ${res.message}`,
                        );
                      }
                    });
                  }}
                  disabled={!enterpriseEnabled}
                  text="Test connection"
                  styleVariant={ButtonStyleVariant.PRIMARY}
                ></Button>
              </div>
            </>
          ) : null
        ) : (
          <p id="sync-not-supported">
            {localLL.form.directory_sync_settings.notSupported()}
          </p>
        )}
      </div>
    </section>
  );
};
