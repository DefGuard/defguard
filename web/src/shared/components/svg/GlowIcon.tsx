import type { SVGProps } from 'react';
const SvgGlowIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    xmlSpace="preserve"
    style={{
      fillRule: 'evenodd',
      clipRule: 'evenodd',
      strokeLinejoin: 'round',
      strokeMiterlimit: 2,
    }}
    viewBox="0 0 24 24"
    {...props}
  >
    <path
      d="M24 5.99A5.99 5.99 0 0 0 18.01 0H5.99A5.99 5.99 0 0 0 0 5.99v12.02A5.99 5.99 0 0 0 5.99 24h12.02A5.99 5.99 0 0 0 24 18.01V5.99Z"
      style={{
        fill: 'url(#glow-icon_svg__a)',
      }}
    />
    <path
      d="M17.917 18.015A8.411 8.411 0 0 1 12 20.437c-2.3 0-4.385-.92-5.907-2.412a9.59 9.59 0 0 1 11.824-.01Zm.098-.098A8.411 8.411 0 0 0 20.437 12a8.41 8.41 0 0 0-2.411-5.906c-2.723 3.465-2.726 8.366-.011 11.823ZM17.93 5.998a9.592 9.592 0 0 1-11.85-.01A8.411 8.411 0 0 1 12 3.562a8.41 8.41 0 0 1 5.93 2.436ZM5.988 6.08A8.41 8.41 0 0 0 3.563 12c0 2.312.929 4.406 2.435 5.93a9.592 9.592 0 0 0-.01-11.85Z"
      style={{
        fill: 'url(#glow-icon_svg__b)',
      }}
    />
    <path
      d="M17.917 18.015A8.411 8.411 0 0 1 12 20.437c-2.3 0-4.385-.92-5.907-2.412a9.59 9.59 0 0 1 11.824-.01Zm.098-.098A8.411 8.411 0 0 0 20.437 12a8.41 8.41 0 0 0-2.411-5.906c-2.723 3.465-2.726 8.366-.011 11.823ZM17.93 5.998a9.592 9.592 0 0 1-11.85-.01A8.411 8.411 0 0 1 12 3.562a8.41 8.41 0 0 1 5.93 2.436ZM5.988 6.08A8.41 8.41 0 0 0 3.563 12c0 2.312.929 4.406 2.435 5.93a9.592 9.592 0 0 0-.01-11.85Z"
      style={{
        fill: 'url(#glow-icon_svg__c)',
      }}
    />
    <defs>
      <radialGradient
        id="glow-icon_svg__a"
        cx={0}
        cy={0}
        r={1}
        gradientTransform="matrix(20.6316 20 -19.9391 20.5687 1.403 2.105)"
        gradientUnits="userSpaceOnUse"
      >
        <stop
          offset={0}
          style={{
            stopColor: '#8000ff',
            stopOpacity: 1,
          }}
        />
        <stop
          offset={0.51}
          style={{
            stopColor: '#a732d6',
            stopOpacity: 1,
          }}
        />
        <stop
          offset={1}
          style={{
            stopColor: '#ef79ff',
            stopOpacity: 1,
          }}
        />
      </radialGradient>
      <radialGradient
        id="glow-icon_svg__c"
        cx={0}
        cy={0}
        r={1}
        gradientTransform="rotate(90 0 12) scale(8.4375)"
        gradientUnits="userSpaceOnUse"
      >
        <stop
          offset={0}
          style={{
            stopColor: '#fff',
            stopOpacity: 0,
          }}
        />
        <stop
          offset={0.91}
          style={{
            stopColor: '#fff',
            stopOpacity: 0,
          }}
        />
        <stop
          offset={1}
          style={{
            stopColor: '#fff',
            stopOpacity: 1,
          }}
        />
      </radialGradient>
      <linearGradient
        id="glow-icon_svg__b"
        x1={0}
        x2={1}
        y1={0}
        y2={0}
        gradientTransform="matrix(0 16.875 -16.875 0 12 3.563)"
        gradientUnits="userSpaceOnUse"
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
            stopOpacity: 0.7,
          }}
        />
      </linearGradient>
    </defs>
  </svg>
);
export default SvgGlowIcon;
