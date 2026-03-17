import { useMemo } from 'react';
import { IconKind } from '../../../../defguard-ui/components/Icon';
import { IconButtonMenu } from '../../../../defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../defguard-ui/components/Menu/types';
import { useTheme } from '../../../../hooks/theme/useTheme';

export const TopBarTheme = () => {
  const { theme, changeTheme } = useTheme();

  const currentIcon = useMemo(() => {
    switch (theme) {
      case 'dark':
        return IconKind.DarkTheme;
      case 'light':
        return IconKind.LightTheme;
    }
  }, [theme]);

  const menu = useMemo(
    (): MenuItemsGroup[] => [
      {
        items: [
          {
            text: 'Light',
            icon: 'light-theme',
            onClick: () => {
              changeTheme('light');
            },
          },
          {
            text: 'Dark',
            icon: 'dark-theme',
            onClick: () => {
              changeTheme('dark');
            },
          },
        ],
      },
    ],
    [changeTheme],
  );

  return (
    <div className="top-bar-theme">
      <IconButtonMenu icon={currentIcon} menuItems={menu} />
    </div>
  );
};
