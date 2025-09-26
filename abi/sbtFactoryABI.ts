export const sbtFactoryABI = [
  "function registerSbtCollection(address sbt_address, string calldata name, string calldata symbol) external",
  "function getIssuerCollections(address issuer) external view returns (string,string,address)[] memory",
  "function isValidSbtContract(address sbt_address) external view returns (bool)",
  "function getTotalCollections() external view returns (uint256)",
];
