import * as React from 'react';
import { SVGProps } from 'react';

const SvgDefguadNavLogoCollapsed = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={20.863} height={44} {...props}>
    <defs>
      <linearGradient
        id="defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-a"
        x1={0.5}
        x2={0.5}
        y2={1}
        gradientUnits="objectBoundingBox"
      >
        <stop offset={0} stopColor="#2accff" />
        <stop offset={1} stopColor="#0071d4" />
      </linearGradient>
      <style>
        {
          '\n      .defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-a{fill:none}.defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-b{fill:url(#defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-a)}\n    '
        }
      </style>
    </defs>
    <path
      className="defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-a"
      d="m13.909 21.976-3.477-2.007-3.477 2.007 3.477 2.008Z"
    />
    <path
      className="defguad-nav-logo-collapsed_svg__defguad-nav-logo-collapsed-b"
      d="M20.863 17.962V2.001l-3.477-2v7.965l-6.954-4-10.431 6v24.018l10.431 6 6.954-4v4.015l-3.473 2 3.477 2 3.473-2V21.967l-10.431-6-6.954 4v-8.01l6.954-4 6.954 4v4Zm-10.431 2.012 3.477 2-3.477 2-3.477-2Zm0 8 6.954-4v8.006l-6.954 4-6.954-4v-8.006Z"
    />
  </svg>
);

export default SvgDefguadNavLogoCollapsed;
