// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract BeamAiOracle {
    error NotOwner();
    error NotReporter();
    error RequestMissing();
    error AlreadyFulfilled();

    struct OracleRequest {
        address requester;
        bytes32 promptHash;
        uint64 createdAt;
        bool fulfilled;
        uint32 scoreBps;
        bytes32 responseHash;
        string responseURI;
    }

    address public owner;
    uint256 public nextRequestId = 1;
    mapping(address => bool) public isReporter;
    mapping(uint256 => OracleRequest) public requests;

    event OracleRequested(uint256 indexed requestId, address indexed requester, bytes32 indexed promptHash);
    event OracleFulfilled(uint256 indexed requestId, address indexed reporter, bytes32 responseHash, uint32 scoreBps, string responseURI);
    event ReporterSet(address indexed reporter, bool enabled);

    constructor() {
        owner = msg.sender;
        isReporter[msg.sender] = true;
        emit ReporterSet(msg.sender, true);
    }

    function requestInference(bytes32 promptHash) external returns (uint256 requestId) {
        requestId = nextRequestId++;
        requests[requestId] = OracleRequest({
            requester: msg.sender,
            promptHash: promptHash,
            createdAt: uint64(block.timestamp),
            fulfilled: false,
            scoreBps: 0,
            responseHash: bytes32(0),
            responseURI: ""
        });
        emit OracleRequested(requestId, msg.sender, promptHash);
    }

    function fulfill(
        uint256 requestId,
        bytes32 responseHash,
        uint32 scoreBps,
        string calldata responseURI
    ) external onlyReporter {
        OracleRequest storage request = requests[requestId];
        if (request.requester == address(0)) revert RequestMissing();
        if (request.fulfilled) revert AlreadyFulfilled();
        request.fulfilled = true;
        request.responseHash = responseHash;
        request.scoreBps = scoreBps;
        request.responseURI = responseURI;
        emit OracleFulfilled(requestId, msg.sender, responseHash, scoreBps, responseURI);
    }

    function setReporter(address reporter, bool enabled) external onlyOwner {
        isReporter[reporter] = enabled;
        emit ReporterSet(reporter, enabled);
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier onlyReporter() {
        if (!isReporter[msg.sender]) revert NotReporter();
        _;
    }
}
