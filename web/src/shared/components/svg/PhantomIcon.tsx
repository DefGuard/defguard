import * as React from 'react';
import { SVGProps } from 'react';

const SvgPhantomIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg
    viewBox="0 0 24 24"
    xmlns="http://www.w3.org/2000/svg"
    xmlSpace="preserve"
    style={{
      fillRule: 'evenodd',
      clipRule: 'evenodd',
      strokeLinejoin: 'round',
      strokeMiterlimit: 2,
    }}
    {...props}
  >
    <circle
      cx={12}
      cy={12}
      r={12}
      style={{
        fill: 'url(#phantom-icon_svg__a)',
      }}
    />
    <path
      d="M20.591 12.146h-2.116c0-4.311-3.507-7.806-7.833-7.806-4.273 0-7.747 3.41-7.832 7.647-.088 4.38 4.036 8.183 8.432 8.183h.553c3.876 0 9.07-3.023 9.882-6.707.15-.679-.388-1.317-1.086-1.317Zm-13.093.192a1.052 1.052 0 0 1-2.103 0v-1.695a1.052 1.052 0 0 1 2.103 0v1.695Zm3.652 0a1.052 1.052 0 0 1-2.103 0v-1.695a1.052 1.052 0 0 1 2.103 0v1.695Z"
      style={{
        fill: 'url(#phantom-icon_svg__b)',
        fillRule: 'nonzero',
      }}
    />
    <defs>
      <linearGradient
        id="phantom-icon_svg__a"
        x1={0}
        y1={0}
        x2={1}
        y2={0}
        gradientUnits="userSpaceOnUse"
        gradientTransform="matrix(0 24 -24 0 12 0)"
      >
        <stop
          offset={0}
          style={{
            stopColor: '#534bb1',
            stopOpacity: 1,
          }}
        />
        <stop
          offset={1}
          style={{
            stopColor: '#551bf9',
            stopOpacity: 1,
          }}
        />
      </linearGradient>
      <linearGradient
        id="phantom-icon_svg__b"
        x1={0}
        y1={0}
        x2={1}
        y2={0}
        gradientUnits="userSpaceOnUse"
        gradientTransform="rotate(90 3.957 8.298) scale(15.8298)"
      >
        <stop
          offset={0}
          style={{
            stopColor: '#fff',
            stopOpacity: 1,
          }}
        />
        <stop
          offset={1}
          style={{
            stopColor: '#fff',
            stopOpacity: 0.82,
          }}
        />
      </linearGradient>
    </defs>
  </svg>
);

export default SvgPhantomIcon;
