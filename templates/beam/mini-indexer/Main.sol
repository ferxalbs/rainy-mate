// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract BeamMiniIndexer {
    error NotOwner();

    address public owner;
    mapping(bytes32 => uint256) public latestCheckpoint;
    mapping(bytes32 => uint256) public latestRecordId;

    event IndexedRecord(
        bytes32 indexed streamId,
        uint256 indexed recordId,
        bytes32 indexed entityId,
        bytes32 payloadHash,
        string payloadURI
    );
    event StreamCheckpoint(bytes32 indexed streamId, uint256 checkpoint);

    constructor() {
        owner = msg.sender;
    }

    function emitRecord(
        bytes32 streamId,
        bytes32 entityId,
        bytes32 payloadHash,
        string calldata payloadURI
    ) external onlyOwner returns (uint256 recordId) {
        recordId = ++latestRecordId[streamId];
        emit IndexedRecord(streamId, recordId, entityId, payloadHash, payloadURI);
    }

    function setCheckpoint(bytes32 streamId, uint256 checkpoint) external onlyOwner {
        latestCheckpoint[streamId] = checkpoint;
        emit StreamCheckpoint(streamId, checkpoint);
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }
}
