import type { SVGProps } from 'react';
const SvgIconCheckmarkWhiteBig = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={36}
    height={36}
    viewBox="0 0 36 36"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h36v36H0z"
          style={{
            opacity: 0,
            fill: '#fff',
          }}
          transform="translate(.203 .203)"
        />
      </clipPath>
      <style>{'.c{fill:#fff}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="rotate(90 18.203 18)"
    >
      <rect
        width={19.857}
        height={3.31}
        className="c"
        rx={1.655}
        transform="rotate(45 -.06 17.375)"
      />
      <rect
        width={13.238}
        height={3.31}
        className="c"
        rx={1.655}
        transform="rotate(-45 39.078 -4.59)"
      />
    </g>
  </svg>
);
export default SvgIconCheckmarkWhiteBig;
