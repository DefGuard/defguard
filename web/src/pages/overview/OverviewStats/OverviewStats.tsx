import './style.scss';

import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import millify from 'millify';
import { forwardRef, ReactNode, useId, useMemo } from 'react';
import AutoSizer from 'react-virtualized-auto-sizer';

import { useI18nContext } from '../../../i18n/i18n-react';
import IconPacketsIn from '../../../shared/components/svg/IconPacketsIn';
import IconPacketsOut from '../../../shared/components/svg/IconPacketsOut';
import { Card } from '../../../shared/defguard-ui/components/Layout/Card/Card';
import { NetworkSpeed } from '../../../shared/defguard-ui/components/Layout/NetworkSpeed/NetworkSpeed';
import { NetworkDirection } from '../../../shared/defguard-ui/components/Layout/NetworkSpeed/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { WireguardNetworkStats } from '../../../shared/types';
import { useOverviewStore } from '../hooks/store/useOverviewStore';
import { NetworkUsageChart } from '../OverviewConnectedUsers/shared/components/NetworkUsageChart/NetworkUsageChart';
import { networkTrafficToChartData } from './utils';

interface Props {
  networkStats: WireguardNetworkStats;
}

export const OverviewStats = forwardRef<HTMLDivElement, Props>(
  ({ networkStats }, ref) => {
    const filterValue = useOverviewStore((state) => state.statsFilter);
    const peakDownload = useMemo(() => {
      const sorted = orderBy(networkStats.transfer_series, (stats) => stats.download, [
        'desc',
      ]);
      return sorted[0]?.download ?? 0;
    }, [networkStats.transfer_series]);
    const peakUpload = useMemo(() => {
      const sorted = orderBy(networkStats.transfer_series, ['upload'], ['desc']);
      return sorted[0]?.upload ?? 0;
    }, [networkStats.transfer_series]);
    const { LL } = useI18nContext();
    const localLL = LL.networkOverview.stats;

    const chartData = useMemo(
      () => networkTrafficToChartData(networkStats.transfer_series, filterValue),
      [filterValue, networkStats.transfer_series],
    );

    const info = useMemo(
      (): InfoProps[] => [
        {
          key: 'currently-active-users',
          count: networkStats.current_active_users,
          icon: <CurrentActiveUsersIcon />,
          title: localLL.currentlyActiveUsers(),
          subTitle: localLL.totalUserDevices({
            count: networkStats.current_active_users,
          }),
        },
        {
          key: 'current-active-network-devices',
          title: localLL.currentlyActiveNetworkDevices(),
          icon: <CurrentActiveNetworkDevicesIcon />,
          count: networkStats.current_active_network_devices,
        },
        {
          key: 'active-users-icon',
          title: localLL.activeUsersFilter({
            hour: filterValue,
          }),
          count: networkStats.active_users,
          icon: <ActiveUsersIcon />,
          subTitle: localLL.totalUserDevices({
            count: networkStats.active_user_devices,
          }),
        },
        {
          key: 'active-network-devices',
          title: localLL.activeNetworkDevices({
            hour: filterValue,
          }),
          icon: <ActiveNetworkDevicesIcon />,
          count: networkStats.current_active_network_devices,
        },
      ],
      [
        filterValue,
        localLL,
        networkStats.active_user_devices,
        networkStats.active_users,
        networkStats.current_active_network_devices,
        networkStats.current_active_users,
      ],
    );

    return (
      <div className="overview-network-stats" ref={ref}>
        <Card className="summary">
          {info.map((info) => (
            <InfoContainer {...info} key={info.key} />
          ))}
          <div className="info network-usage" data-testid="network-usage">
            <span className="info-title">{LL.networkOverview.stats.networkUsage()}</span>
            <div className="network-usage-track">
              <div className="network-usage">
                <span>
                  <IconPacketsIn /> {LL.networkOverview.stats.in()}
                </span>
                <NetworkSpeed
                  speedValue={networkStats.download}
                  direction={NetworkDirection.DOWNLOAD}
                  data-testid="download"
                />
              </div>
              <div className="network-usage">
                <span>
                  <IconPacketsOut /> {LL.networkOverview.stats.out()}
                </span>
                <NetworkSpeed
                  speedValue={networkStats.upload}
                  direction={NetworkDirection.UPLOAD}
                  data-testid="upload"
                />
              </div>
            </div>
          </div>
        </Card>
        <Card className="activity-graph">
          <header>
            <h3>{LL.networkOverview.stats.activityIn({ hour: filterValue })}</h3>
            <div className="peaks">
              <span>{LL.networkOverview.stats.peak()}</span>
              <div className="network-speed">
                <IconPacketsIn />
                <NetworkSpeed
                  speedValue={peakDownload}
                  direction={NetworkDirection.DOWNLOAD}
                />
              </div>
              <div className="network-speed">
                <IconPacketsOut />
                <NetworkSpeed
                  speedValue={peakUpload}
                  direction={NetworkDirection.UPLOAD}
                />
              </div>
            </div>
          </header>
          <div className="chart">
            <AutoSizer>
              {({ width, height }) => (
                <>
                  {networkStats.transfer_series && (
                    <NetworkUsageChart
                      data={chartData}
                      hideX={false}
                      height={height}
                      width={width}
                      barSize={4}
                      barGap={1}
                    />
                  )}
                </>
              )}
            </AutoSizer>
          </div>
        </Card>
      </div>
    );
  },
);

type InfoProps = {
  icon: ReactNode;
  title: string;
  subTitle?: string;
  count: number;
  key: string | number;
};

const InfoContainer = ({ count, icon, subTitle, title }: InfoProps) => {
  return (
    <div className={clsx('info')}>
      <p className="info-title">{title}</p>
      <div className="info-track">
        {icon}
        <p className="info-count">
          {millify(count, {
            precision: 0,
          })}
        </p>
      </div>
      {isPresent(subTitle) && <p className="info-sub-title">{subTitle}</p>}
    </div>
  );
};

const CurrentActiveUsersIcon = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="25"
      height="24"
      viewBox="0 0 25 24"
      fill="none"
    >
      <path
        d="M21.131 24C21.008 24 20.8861 23.9758 20.7724 23.9287C20.6587 23.8816 20.5555 23.8126 20.4684 23.7256C20.3814 23.6385 20.3124 23.5352 20.2653 23.4216C20.2182 23.3079 20.194 23.1861 20.194 23.063C20.1919 21.2984 19.49 19.6066 18.2422 18.3589C16.9944 17.1111 15.3026 16.4091 13.538 16.407H12.131C10.3664 16.4091 8.67463 17.1111 7.42684 18.3589C6.17906 19.6066 5.47712 21.2984 5.47501 23.063C5.46706 23.3064 5.3648 23.5371 5.18986 23.7064C5.01491 23.8757 4.78098 23.9704 4.53751 23.9704C4.29404 23.9704 4.0601 23.8757 3.88515 23.7064C3.71021 23.5371 3.60795 23.3064 3.60001 23.063C3.60239 20.8011 4.50192 18.6325 6.10125 17.033C7.70058 15.4334 9.86908 14.5337 12.131 14.531H13.537C15.7988 14.5336 17.9671 15.4333 19.5664 17.0326C21.1657 18.6319 22.0654 20.8003 22.068 23.062C22.0681 23.1851 22.044 23.3071 21.997 23.4209C21.9499 23.5347 21.8809 23.6381 21.7939 23.7252C21.7069 23.8123 21.6036 23.8814 21.4898 23.9286C21.3761 23.9757 21.2541 24 21.131 24Z"
        fill="#899CA8"
      />
      <path
        d="M12.741 12.656C11.4894 12.656 10.266 12.2849 9.22536 11.5895C8.18472 10.8942 7.37364 9.90591 6.89469 8.74962C6.41574 7.59333 6.29042 6.32098 6.53459 5.09347C6.77876 3.86596 7.38145 2.73842 8.26643 1.85343C9.15142 0.968444 10.279 0.365761 11.5065 0.121594C12.734 -0.122574 14.0063 0.00274181 15.1626 0.481693C16.3189 0.960644 17.3072 1.77172 18.0025 2.81235C18.6979 3.85299 19.069 5.07644 19.069 6.328C19.0672 8.00572 18.3999 9.6142 17.2135 10.8005C16.0272 11.9869 14.4187 12.6542 12.741 12.656ZM12.741 1.875C11.8603 1.875 10.9993 2.13617 10.267 2.62547C9.53475 3.11477 8.96401 3.81023 8.62697 4.62391C8.28993 5.43759 8.20175 6.33294 8.37357 7.19674C8.54539 8.06054 8.96949 8.85399 9.59225 9.47675C10.215 10.0995 11.0085 10.5236 11.8723 10.6954C12.7361 10.8673 13.6314 10.7791 14.4451 10.442C15.2588 10.105 15.9542 9.53425 16.4435 8.80196C16.9328 8.06966 17.194 7.20872 17.194 6.328C17.1927 5.1474 16.7231 4.01553 15.8883 3.18072C15.0535 2.34591 13.9216 1.87633 12.741 1.875Z"
        fill="#899CA8"
      />
    </svg>
  );
};

const CurrentActiveNetworkDevicesIcon = () => {
  const maskId = useId();
  return (
    <svg
      width={25}
      height={24}
      viewBox="0 0 25 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M14.8 0H20.8C23.0091 0 24.8 1.79086 24.8 4V20C24.8 22.2091 23.0091 24 20.8 24H14.8C12.5908 24 10.8 22.2091 10.8 20V4C10.8 1.79086 12.5908 0 14.8 0ZM14.8 2C13.6954 2 12.8 2.89543 12.8 4V20C12.8 21.1046 13.6954 22 14.8 22H20.8C21.9046 22 22.8 21.1046 22.8 20V4C22.8 2.89543 21.9046 2 20.8 2H14.8Z"
        fill="#899CA8"
      />
      <mask
        id={maskId}
        style={{
          maskType: 'alpha',
        }}
        maskUnits="userSpaceOnUse"
        x={0}
        y={0}
        width={25}
        height={19}
      >
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M6.42613 2C5.46482 2.00129 4.54324 2.38373 3.86348 3.06349C3.18372 3.74326 2.80127 4.66483 2.79999 5.62614V13.3119C2.80127 14.2729 3.18361 15.1942 3.86319 15.8738C4.54264 16.5533 5.46376 16.9356 6.42462 16.937C6.42512 16.937 6.42562 16.937 6.42612 16.937H19.1729C19.1734 16.937 19.1739 16.937 19.1744 16.937C20.1354 16.9356 21.0566 16.5532 21.7363 15.8737C22.4159 15.1942 22.7984 14.2729 22.8 13.3119C22.8 13.3114 22.8 13.311 22.8 13.3105V5.62713C22.8 5.62663 22.8 5.62613 22.8 5.62563C22.7986 4.66451 22.4161 3.74314 21.7365 3.06349C21.0567 2.38373 20.1352 2.00129 19.1738 2H6.42613ZM19.175 0H6.42499C4.93363 0.00158824 3.50381 0.594731 2.44927 1.64928C1.39472 2.70383 0.801576 4.13364 0.799988 5.625V13.313C0.801576 14.8041 1.39461 16.2337 2.44897 17.288C3.50333 18.3424 4.9329 18.9354 6.42399 18.937H19.175C20.6662 18.9354 22.0958 18.3424 23.1504 17.2881C24.2049 16.2337 24.7981 14.8042 24.8 13.313V5.625C24.7984 4.13364 24.2053 2.70383 23.1507 1.64928C22.0962 0.594731 20.6663 0.00158824 19.175 0Z"
          fill="#899CA8"
        />
      </mask>
      <g mask={`url(#${maskId})`}>
        <rect
          width={10}
          height={23}
          transform="matrix(-1 0 0 1 9.79999 -2)"
          fill="#899CA8"
        />
      </g>
      <path
        d="M4.79999 23C4.79999 22.4477 5.2477 22 5.79999 22H9.79999V24H5.79999C5.2477 24 4.79999 23.5523 4.79999 23Z"
        fill="#899CA8"
      />
      <circle cx={17.8} cy={19} r={1} fill="#899CA8" />
    </svg>
  );
};

const ActiveUsersIcon = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={24}
      height={24}
      viewBox="0 0 24 24"
      fill="none"
    >
      <path
        d="M9.14038 12.6563C10.392 12.6563 11.6156 12.2851 12.6563 11.5897C13.697 10.8943 14.5081 9.90594 14.987 8.74957C15.466 7.5932 15.5912 6.32077 15.347 5.09321C15.1027 3.86564 14.4999 2.73808 13.6148 1.85312C12.7297 0.96815 11.602 0.365533 10.3744 0.121474C9.14672 -0.122585 7.87429 0.00287559 6.71796 0.48199C5.56164 0.961104 4.57337 1.77235 3.87814 2.81314C3.1829 3.85393 2.81192 5.0775 2.81212 6.32913C2.81423 8.00671 3.48167 9.61496 4.66802 10.8011C5.85437 11.9872 7.46276 12.6544 9.14038 12.6563ZM9.14038 1.87504C10.0211 1.87504 10.8821 2.13621 11.6144 2.62552C12.3468 3.11483 12.9175 3.81031 13.2546 4.62401C13.5916 5.4377 13.6798 6.33307 13.508 7.19689C13.3362 8.0607 12.912 8.85417 12.2893 9.47694C11.6665 10.0997 10.873 10.5238 10.0092 10.6957C9.14532 10.8675 8.24993 10.7793 7.43622 10.4423C6.6225 10.1052 5.92701 9.53444 5.43769 8.80214C4.94837 8.06983 4.68719 7.20887 4.68719 6.32813C4.68878 5.14768 5.1585 4.01604 5.99332 3.18143C6.82814 2.34681 7.9599 1.87736 9.14038 1.87604V1.87504Z"
        fill="#899CA8"
      />
      <path
        d="M8.67136 14.5313C6.39492 14.5287 4.20894 15.4223 2.58621 17.0188C0.963474 18.6152 0.0344263 20.7864 0 23.0625C0 23.3111 0.0987761 23.5496 0.274599 23.7254C0.450421 23.9012 0.688888 24 0.937539 24C1.18619 24 1.42466 23.9012 1.60048 23.7254C1.7763 23.5496 1.87508 23.3111 1.87508 23.0625C1.90892 21.2828 2.64076 19.5877 3.91283 18.3426C5.18491 17.0975 6.89533 16.4021 8.67536 16.4063C8.92401 16.4063 9.16248 16.3076 9.3383 16.1317C9.51412 15.9559 9.6129 15.7175 9.6129 15.4688C9.6129 15.2202 9.51412 14.9817 9.3383 14.8059C9.16248 14.6301 8.92401 14.5313 8.67536 14.5313H8.67136Z"
        fill="#899CA8"
      />
      <path
        d="M19.2638 16.6883H18.6078V15.7043C18.6078 15.4557 18.509 15.2172 18.3332 15.0414C18.1574 14.8656 17.9189 14.7668 17.6702 14.7668C17.4216 14.7668 17.1831 14.8656 17.0073 15.0414C16.8315 15.2172 16.7327 15.4557 16.7327 15.7043V17.6263C16.7327 17.8749 16.8314 18.1132 17.0071 18.2889C17.1829 18.4647 17.4212 18.5634 17.6697 18.5634H19.2628C19.5114 18.5634 19.7499 18.4646 19.9258 18.2888C20.1016 18.113 20.2003 17.8745 20.2003 17.6259C20.2003 17.3772 20.1016 17.1388 19.9258 16.9629C19.7499 16.7871 19.5114 16.6883 19.2628 16.6883H19.2638Z"
        fill="#899CA8"
      />
      <path
        d="M17.6717 11.2972C16.4201 11.2972 15.1966 11.6684 14.1559 12.3637C13.1153 13.0591 12.3041 14.0474 11.8252 15.2037C11.3462 16.36 11.2209 17.6324 11.4651 18.8599C11.7092 20.0875 12.3119 21.215 13.197 22.1C14.082 22.985 15.2096 23.5877 16.4372 23.8319C17.6647 24.0761 18.9371 23.9508 20.0935 23.4718C21.2498 22.9928 22.2381 22.1817 22.9335 21.1411C23.6288 20.1004 24 18.8769 24 17.6254C23.9981 15.9476 23.3308 14.3391 22.1444 13.1527C20.9581 11.9664 19.3495 11.2991 17.6717 11.2972ZM17.6717 22.0785C16.791 22.0785 15.93 21.8173 15.1977 21.328C14.4654 20.8387 13.8946 20.1432 13.5575 19.3295C13.2205 18.5158 13.1323 17.6204 13.3041 16.7566C13.4759 15.8928 13.9001 15.0993 14.5229 14.4766C15.1457 13.8538 15.9391 13.4297 16.803 13.2578C17.6668 13.086 18.5622 13.1742 19.3759 13.5112C20.1896 13.8483 20.8851 14.4191 21.3744 15.1514C21.8637 15.8837 22.1249 16.7446 22.1249 17.6254C22.1236 18.806 21.654 19.9379 20.8191 20.7727C19.9843 21.6075 18.8524 22.0771 17.6717 22.0785Z"
        fill="#899CA8"
      />
    </svg>
  );
};

const ActiveNetworkDevicesIcon = () => {
  const maskId = useId();
  const mask2Id = useId();
  return (
    <svg
      width={25}
      height={24}
      viewBox="0 0 25 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <mask
        id={maskId}
        style={{
          maskType: 'alpha',
        }}
        maskUnits="userSpaceOnUse"
        x={0}
        y={0}
        width={25}
        height={19}
      >
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M5.82609 2C4.86479 2.00129 3.94321 2.38373 3.26344 3.06349C2.58368 3.74326 2.20124 4.66483 2.19995 5.62614V13.3119C2.20124 14.2729 2.58357 15.1942 3.26315 15.8738C3.94261 16.5533 4.86373 16.9356 5.82458 16.937C5.82508 16.937 5.82558 16.937 5.82608 16.937H18.5728C18.5733 16.937 18.5738 16.937 18.5743 16.937C19.5353 16.9356 20.4566 16.5532 21.1362 15.8737C21.8159 15.1942 22.1984 14.2729 22.2 13.3119C22.2 13.3114 22.2 13.311 22.2 13.3105V5.62713C22.2 5.62663 22.2 5.62613 22.2 5.62563C22.1985 4.66451 21.8161 3.74314 21.1365 3.06349C20.4567 2.38373 19.5351 2.00129 18.5738 2H5.82609ZM18.575 0H5.82495C4.3336 0.00158824 2.90378 0.594731 1.84923 1.64928C0.794682 2.70383 0.201539 4.13364 0.199951 5.625V13.313C0.201539 14.8041 0.794576 16.2337 1.84894 17.288C2.9033 18.3424 4.33286 18.9354 5.82395 18.937H18.575C20.0661 18.9354 21.4958 18.3424 22.5503 17.2881C23.6048 16.2337 24.1981 14.8042 24.2 13.313V5.625C24.1984 4.13364 23.6052 2.70383 22.5507 1.64928C21.4961 0.594731 20.0663 0.00158824 18.575 0Z"
          fill="#899CA8"
        />
      </mask>
      <g mask={`url(#${maskId})`}>
        <rect
          width={10}
          height={23}
          transform="matrix(-1 0 0 1 9.19995 -2)"
          fill="#899CA8"
        />
      </g>
      <path
        d="M4.19995 23C4.19995 22.4477 4.64767 22 5.19995 22H9.19995V24H5.19995C4.64767 24 4.19995 23.5523 4.19995 23Z"
        fill="#899CA8"
      />
      <path
        d="M19.463 16.688H18.807V15.704C18.807 15.4553 18.7082 15.2169 18.5324 15.0411C18.3566 14.8653 18.1181 14.7665 17.8695 14.7665C17.6208 14.7665 17.3824 14.8653 17.2066 15.0411C17.0307 15.2169 16.932 15.4553 16.932 15.704V17.626C16.932 17.8745 17.0307 18.1128 17.2064 18.2886C17.3821 18.4643 17.6205 18.563 17.869 18.563H19.462C19.7106 18.563 19.9491 18.4642 20.1249 18.2884C20.3007 18.1126 20.3995 17.8741 20.3995 17.6255C20.3995 17.3769 20.3007 17.1384 20.1249 16.9626C19.9491 16.7868 19.7106 16.688 19.462 16.688H19.463Z"
        fill="#899CA8"
      />
      <path
        d="M17.871 11.297C16.6194 11.297 15.3959 11.6681 14.3553 12.3635C13.3147 13.0588 12.5036 14.0471 12.0247 15.2034C11.5457 16.3597 11.4204 17.632 11.6646 18.8595C11.9087 20.087 12.5114 21.2146 13.3964 22.0996C14.2814 22.9846 15.4089 23.5872 16.6364 23.8314C17.864 24.0756 19.1363 23.9503 20.2926 23.4713C21.4489 22.9924 22.4372 22.1813 23.1325 21.1406C23.8278 20.1 24.199 18.8766 24.199 17.625C24.1971 15.9473 23.5298 14.3388 22.3435 13.1525C21.1572 11.9661 19.5487 11.2989 17.871 11.297ZM17.871 22.078C16.9902 22.078 16.1293 21.8168 15.397 21.3275C14.6647 20.8382 14.094 20.1428 13.7569 19.3291C13.4199 18.5154 13.3317 17.6201 13.5035 16.7563C13.6753 15.8925 14.0995 15.099 14.7222 14.4763C15.345 13.8535 16.1384 13.4294 17.0022 13.2576C17.866 13.0857 18.7614 13.1739 19.5751 13.511C20.3887 13.848 21.0842 14.4188 21.5735 15.151C22.0628 15.8833 22.324 16.7443 22.324 17.625C22.3226 18.8056 21.8531 19.9375 21.0182 20.7723C20.1834 21.6071 19.0516 22.0767 17.871 22.078Z"
        fill="#899CA8"
      />
      <mask
        id={mask2Id}
        style={{
          maskType: 'alpha',
        }}
        maskUnits="userSpaceOnUse"
        x={10}
        y={0}
        width={15}
        height={24}
      >
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M14.2 0H20.2C22.4091 0 24.2 1.79086 24.2 4V20C24.2 22.2091 22.4091 24 20.2 24H14.2C11.9908 24 10.2 22.2091 10.2 20V4C10.2 1.79086 11.9908 0 14.2 0ZM14.2 2C13.0954 2 12.2 2.89543 12.2 4V20C12.2 21.1046 13.0954 22 14.2 22H20.2C21.3045 22 22.2 21.1046 22.2 20V4C22.2 2.89543 21.3045 2 20.2 2H14.2Z"
          fill="#899CA8"
        />
      </mask>
      <g mask={`url(#${mask2Id})`}>
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M24.2 0.019165H10.2V14.0937C11.5368 11.1945 14.4689 9.18225 17.8709 9.18225C20.3909 9.18225 22.653 10.2863 24.2 12.037V0.019165Z"
          fill="#899CA8"
        />
      </g>
    </svg>
  );
};
