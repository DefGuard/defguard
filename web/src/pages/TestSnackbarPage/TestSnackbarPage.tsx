import { useCallback, useState } from 'react';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import './style.scss';
import { m } from '../../paraglide/messages';
import { Input } from '../../shared/defguard-ui/components/Input/Input';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { SnackbarVariant } from '../../shared/defguard-ui/providers/snackbar/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';

const initialText = m.test_placeholder();

export const TestSnackbarPage = () => {
  const [loading, setLoading] = useState(false);
  const [snackbarText, setSnackbarContent] = useState<string>(initialText);

  const handleActionTest = useCallback(() => {
    window.alert('Action test callback.');
  }, []);

  const handleLoadingSpawn = useCallback(() => {
    const anchor = Snackbar.loading(snackbarText, 'loading-snack-test-1');
    setLoading(true);
    setTimeout(() => {
      anchor.update({
        text: 'Updated by context after 5 seconds',
      });
    }, 5000);
    setTimeout(() => {
      anchor.dismiss();
      setLoading(false);
    }, 10000);
  }, [snackbarText]);

  return (
    <div id="test-snackbar">
      <Input
        notNull
        value={snackbarText}
        label="Snackbar content"
        onChange={(val) => {
          if (typeof val === 'string') {
            setSnackbarContent(val);
          }
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        text="Test Default"
        onClick={() => {
          Snackbar.default(snackbarText);
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        variant="outlined"
        text="Test Success"
        onClick={() => {
          Snackbar.success(snackbarText);
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        variant="critical"
        text="Test Error"
        onClick={() => {
          Snackbar.error(snackbarText);
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        loading={loading}
        variant="outlined"
        text="Test Loading"
        onClick={() => {
          handleLoadingSpawn();
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        variant="primary"
        text="Test dismissible"
        onClick={() => {
          Snackbar.custom({
            id: 'snack-text-dismissible',
            variant: SnackbarVariant.Success,
            dismissible: true,
            text: snackbarText,
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        variant="outlined"
        text="Test action"
        onClick={() => {
          Snackbar.custom({
            id: 'snack-text-dismissible',
            variant: SnackbarVariant.Success,
            text: snackbarText,
            action: {
              text: `Click me`,
              onClick: handleActionTest,
            },
          });
        }}
      />
    </div>
  );
};
