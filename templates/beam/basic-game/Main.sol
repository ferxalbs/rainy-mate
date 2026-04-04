// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract BeamArcadeArena {
    error NotOwner();
    error SessionAlreadyRecorded();
    error InvalidWinner();
    error NoRewards();

    address public owner;
    uint256 public totalSessions;
    uint256 public rewardPerWin = 0.01 ether;

    mapping(bytes32 => bool) public sessionRecorded;
    mapping(address => uint256) public bestScore;
    mapping(address => uint256) public wins;
    mapping(address => uint256) public pendingRewards;

    event SessionReported(bytes32 indexed sessionId, address indexed player, uint256 score, bool won);
    event RewardClaimed(address indexed player, uint256 amount);
    event RewardPoolFunded(address indexed sender, uint256 amount);

    constructor() {
        owner = msg.sender;
    }

    function reportSession(bytes32 sessionId, address player, uint256 score, bool won) external onlyOwner {
        if (sessionRecorded[sessionId]) revert SessionAlreadyRecorded();
        if (player == address(0)) revert InvalidWinner();
        sessionRecorded[sessionId] = true;
        totalSessions += 1;

        if (score > bestScore[player]) {
            bestScore[player] = score;
        }
        if (won) {
            wins[player] += 1;
            pendingRewards[player] += rewardPerWin;
        }
        emit SessionReported(sessionId, player, score, won);
    }

    function claimRewards() external {
        uint256 amount = pendingRewards[msg.sender];
        if (amount == 0) revert NoRewards();
        pendingRewards[msg.sender] = 0;
        (bool sent, ) = payable(msg.sender).call{value: amount}("");
        require(sent, "reward transfer failed");
        emit RewardClaimed(msg.sender, amount);
    }

    function fundRewardPool() external payable {
        emit RewardPoolFunded(msg.sender, msg.value);
    }

    function setRewardPerWin(uint256 nextRewardPerWin) external onlyOwner {
        rewardPerWin = nextRewardPerWin;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }
}
