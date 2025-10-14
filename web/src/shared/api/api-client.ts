/** biome-ignore-all lint/suspicious/noExplicitAny: too generic to bother typing this strictly */
/** biome-ignore-all lint/suspicious/noConfusingVoidType: same */
/** biome-ignore-all lint/complexity/noBannedTypes: same */
import type { QueryKey } from '@tanstack/react-query';
import axios from 'axios';
import qs from 'qs';

const client = axios.create({
  baseURL: '/api/v1',
  headers: { 'Content-Type': 'application/json' },
  paramsSerializer: {
    serialize: (params) =>
      qs.stringify(params, {
        arrayFormat: 'repeat',
      }),
  },
});

type ApiResponse<T> = {
  data: T;
  status: number;
  invalidateKeys?: QueryKey[];
};

const RequestMethod = {
  Get: 'get',
  Post: 'post',
  Patch: 'patch',
  Delete: 'delete',
} as const;

type RequestMethodValue = (typeof RequestMethod)[keyof typeof RequestMethod];

type UrlLike<Path = never> = string | ((path: Path) => string);
type PathFromUrl<U> = U extends (path: infer P) => string ? P : never;

type PathProp<Path> = [Path] extends [never] ? { path?: never } : { path: Path };

type ParamsProp<P> = [P] extends [never]
  ? { params?: never }
  : [P] extends [void]
    ? { params?: undefined }
    : { params: P };

type GetDeleteProps<Params, Path> = PathProp<Path> &
  ParamsProp<Params> & {
    data?: never;
    abortSignal?: AbortSignal;
  };

type PostPatchProps<Body, Params, Path> = PathProp<Path> & {
  data: Body;
} & ParamsProp<Params> & {
    abortSignal?: AbortSignal;
  };

type RequestHandle<Fn> = {
  callbackFn: Fn;
  invalidateKeys?: QueryKey[];
};

type Cfg<M extends RequestMethodValue, U extends UrlLike<any>> = {
  method: M;
  url: U;
  invalidateKeys?: QueryKey[];
};

function createRequest<U extends UrlLike<any>, Params = void, Res = unknown>(
  cfg: Cfg<typeof RequestMethod.Get, U> | Cfg<typeof RequestMethod.Delete, U>,
): RequestHandle<
  (props: GetDeleteProps<Params, PathFromUrl<U>>) => Promise<ApiResponse<Res>>
>;

function createRequest<
  U extends UrlLike<any>,
  Body = unknown,
  Res = unknown,
  Params = never,
>(
  cfg: Cfg<typeof RequestMethod.Post, U> | Cfg<typeof RequestMethod.Patch, U>,
): RequestHandle<
  (props: PostPatchProps<Body, Params, PathFromUrl<U>>) => Promise<ApiResponse<Res>>
>;

function createRequest(
  cfg: Cfg<RequestMethodValue, UrlLike<any>>,
): RequestHandle<(props: any) => Promise<ApiResponse<any>>> {
  const callbackFn = async (props: any = {}): Promise<ApiResponse<any>> => {
    const { abortSignal, path, ...rest } = props ?? {};
    const method = cfg.method;

    if ((method === 'get' || method === 'delete') && 'data' in rest) {
      throw new Error(`[${method.toUpperCase()}] must not include 'data'.`);
    }
    if ((method === 'post' || method === 'patch') && !('data' in rest)) {
      throw new Error(`[${method.toUpperCase()}] requires 'data'.`);
    }

    const finalUrl =
      typeof cfg.url === 'function'
        ? (cfg.url as (p: any) => string)(path)
        : (cfg.url as string);

    const axiosRes = await client.request({
      url: finalUrl,
      method,
      ...(rest.params !== undefined ? { params: rest.params } : {}),
      ...(rest.data !== undefined ? { data: rest.data } : {}),
      signal: abortSignal,
    });

    return {
      data: axiosRes.data,
      status: axiosRes.status,
      invalidateKeys: cfg.invalidateKeys,
    };
  };

  return { callbackFn, invalidateKeys: cfg.invalidateKeys };
}

type HelperOpts = { invalidateKeys?: QueryKey[] };

export function get<Res = unknown, Params = void>(
  url: string,
  opts?: HelperOpts,
): RequestHandle<(props: GetDeleteProps<Params, never>) => Promise<ApiResponse<Res>>>;
export function get<Res = unknown, Params = void, P = unknown>(
  url: (path: P) => string,
  opts?: HelperOpts,
): RequestHandle<(props: GetDeleteProps<Params, P>) => Promise<ApiResponse<Res>>>;
// implementation signature must be compatible with both overloads:
export function get(url: UrlLike<unknown>, opts?: HelperOpts) {
  return createRequest({
    method: RequestMethod.Get,
    url: url as UrlLike<any>,
    invalidateKeys: opts?.invalidateKeys,
  }) as any;
}

export function del<Res = unknown, Params = void>(
  url: string,
  opts?: HelperOpts,
): RequestHandle<(props: GetDeleteProps<Params, never>) => Promise<ApiResponse<Res>>>;
export function del<Res = unknown, Params = void, P = unknown>(
  url: (path: P) => string,
  opts?: HelperOpts,
): RequestHandle<(props: GetDeleteProps<Params, P>) => Promise<ApiResponse<Res>>>;
export function del(url: UrlLike<unknown>, opts?: HelperOpts) {
  return createRequest({
    method: RequestMethod.Delete,
    url: url as UrlLike<any>,
    invalidateKeys: opts?.invalidateKeys,
  }) as any;
}

export function post<Body = unknown, Res = unknown, Params = never>(
  url: string,
  opts?: HelperOpts,
): RequestHandle<
  (props: PostPatchProps<Body, Params, never>) => Promise<ApiResponse<Res>>
>;
export function post<Body = unknown, Res = unknown, Params = never, P = unknown>(
  url: (path: P) => string,
  opts?: HelperOpts,
): RequestHandle<(props: PostPatchProps<Body, Params, P>) => Promise<ApiResponse<Res>>>;
export function post(url: UrlLike<unknown>, opts?: HelperOpts) {
  return createRequest({
    method: RequestMethod.Post,
    url: url as UrlLike<any>,
    invalidateKeys: opts?.invalidateKeys,
  }) as any;
}

export function patch<Body = unknown, Res = unknown, Params = never>(
  url: string,
  opts?: HelperOpts,
): RequestHandle<
  (props: PostPatchProps<Body, Params, never>) => Promise<ApiResponse<Res>>
>;
export function patch<Body = unknown, Res = unknown, Params = never, P = unknown>(
  url: (path: P) => string,
  opts?: HelperOpts,
): RequestHandle<(props: PostPatchProps<Body, Params, P>) => Promise<ApiResponse<Res>>>;
export function patch(url: UrlLike<unknown>, opts?: HelperOpts) {
  return createRequest({
    method: RequestMethod.Patch,
    url: url as UrlLike<any>,
    invalidateKeys: opts?.invalidateKeys,
  }) as any;
}
