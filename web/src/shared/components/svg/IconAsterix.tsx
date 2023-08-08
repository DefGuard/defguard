import type { SVGProps } from 'react';
const SvgIconAsterix = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#cbd3d8',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#cbd3d8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="rotate(90 11 11)"
    >
      <rect
        width={12}
        height={2}
        className="c"
        rx={1}
        transform="rotate(60 -.16 10.33)"
      />
      <rect
        width={12}
        height={2}
        className="c"
        rx={1}
        transform="rotate(-60 17.16 1.67)"
      />
      <rect width={12} height={2} className="c" rx={1} transform="rotate(180 8.5 6)" />
    </g>
  </svg>
);
export default SvgIconAsterix;
