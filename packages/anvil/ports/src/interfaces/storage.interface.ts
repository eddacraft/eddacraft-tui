/**
 * Storage interface definitions
 *
 * Defines the contract for storage providers.
 */

/**
 * Storage provider interface
 */
export interface IStorageProvider {
  /** Read a file as string */
  read(path: string): Promise<string>;

  /** Read a file as buffer */
  readBuffer(path: string): Promise<Buffer>;

  /** Write content to a file */
  write(path: string, content: string | Buffer): Promise<void>;

  /** Check if a file exists */
  exists(path: string): Promise<boolean>;

  /** Delete a file */
  delete(path: string): Promise<void>;

  /** List files in a directory */
  list(directory: string): Promise<string[]>;

  /** Create a directory */
  mkdir(path: string, recursive?: boolean): Promise<void>;
}
