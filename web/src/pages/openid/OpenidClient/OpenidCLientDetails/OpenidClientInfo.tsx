import React, { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import { OpenidClient } from '../../../../shared/types';
interface Props {
  client: OpenidClient;
}

interface Detail {
  label: string;
  value?: string;
  hidden?: boolean;
  titleCase?: boolean;
}

interface DetailGroup {
  title: string;
  details: Detail[];
  hidden?: boolean;
  customDetails?: React.ReactNode;
  adminOnly?: boolean;
}

const OpenidClientInfo: React.FC<Props> = ({ client }) => {
  const { t } = useTranslation('en');
  const details: DetailGroup[] = useMemo((): DetailGroup[] => {
    let res: DetailGroup[] = [
      {
        title: 'App information',
        details: [
          {
            label: t('openid.app.name'),
            value: `${client.name}`,
            titleCase: true,
          },
          {
            label: t('openid.app.description'),
            value: client.description,
          },
          {
            label: t('openid.app.homeUrl'),
            value: client.home_url,
          },
          {
            label: t('openid.app.redirectUri'),
            value: client.redirect_uri,
          },
          {
            label: t('openid.app.clientId'),
            value: client.client_id,
          },
          {
            label: t('openid.app.clientSecret'),
            value: client.client_secret,
          },
        ],
      },
    ];
    res = res.filter((group) => !group.hidden);
    res = res.filter(
      (group) =>
        group.details.filter((detail) => detail.value && !detail.hidden)
          .length || group.customDetails
    );
    res = res.map((group) => ({
      ...group,
      details: group.details.filter((detail) => detail.value && !detail.hidden),
    }));
    return res;
  }, [client, t]);

  return (
    <section className="info">
      {details.map((group) => {
        return (
          <div className="detail-group" key={group.title}>
            <span className="group-title">{group.title}</span>
            <div className="details">
              {typeof group.customDetails === 'undefined'
                ? group.details.map((detail) => (
                    <div key={detail.label.toLowerCase()} className="detail">
                      <label>{detail.label}:</label>
                      <span className={detail.titleCase ? 'title-case' : ''}>
                        {detail.value}
                      </span>
                    </div>
                  ))
                : group.customDetails}
            </div>
          </div>
        );
      })}
    </section>
  );
};

export default OpenidClientInfo;
