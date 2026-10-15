EXPECTED = {
    "S1AP-CommonDataTypes": {
        "extensibility-implied": False,
        "imports": {},
        "object-classes": {},
        "object-sets": {},
        "tags": "AUTOMATIC",
        "types": {
            "Criticality": {
                "type": "ENUMERATED",
                "values": [("reject", 0), ("ignore", 1), ("notify", 2)],
            },
            "Presence": {
                "type": "ENUMERATED",
                "values": [("optional", 0), ("conditional", 1), ("mandatory", 2)],
            },
            "PrivateIE-ID": {
                "members": [
                    {"name": "local", "restricted-to": [(0, 65535)], "type": "INTEGER"},
                    {"name": "global", "type": "OBJECT IDENTIFIER"},
                ],
                "type": "CHOICE",
            },
            "ProcedureCode": {"restricted-to": [(0, 255)], "type": "INTEGER"},
            "ProtocolExtensionID": {"restricted-to": [(0, 65535)], "type": "INTEGER"},
            "ProtocolIE-ID": {"restricted-to": [(0, 65535)], "type": "INTEGER"},
            "TriggeringMessage": {
                "type": "ENUMERATED",
                "values": [
                    ("initiating-message", 0),
                    ("successful-outcome", 1),
                    ("unsuccessfull-outcome", 2),
                ],
            },
        },
        "values": {},
    },
    "S1AP-Constants": {
        "extensibility-implied": False,
        "imports": {"S1AP-CommonDataTypes": ["ProcedureCode", "ProtocolIE-ID"]},
        "object-classes": {},
        "object-sets": {},
        "tags": "AUTOMATIC",
        "types": {},
        "values": {
            "id-Additional-GUTI": {"type": "ProtocolIE-ID", "value": 224},
            "id-AdditionalCSFallbackIndicator": {"type": "ProtocolIE-ID", "value": 187},
            "id-AssistanceDataForPaging": {"type": "ProtocolIE-ID", "value": 211},
            "id-BearerType": {"type": "ProtocolIE-ID", "value": 233},
            "id-Bearers-SubjectToStatusTransfer-Item": {
                "type": "ProtocolIE-ID",
                "value": 89,
            },
            "id-BroadcastCancelledAreaList": {"type": "ProtocolIE-ID", "value": 141},
            "id-BroadcastCompletedAreaList": {"type": "ProtocolIE-ID", "value": 120},
            "id-CE-mode-B-SupportIndicator": {"type": "ProtocolIE-ID", "value": 242},
            "id-CNDomain": {"type": "ProtocolIE-ID", "value": 109},
            "id-CSFallbackIndicator": {"type": "ProtocolIE-ID", "value": 108},
            "id-CSG-Id": {"type": "ProtocolIE-ID", "value": 127},
            "id-CSG-IdList": {"type": "ProtocolIE-ID", "value": 128},
            "id-CSGMembershipInfo": {"type": "ProtocolIE-ID", "value": 226},
            "id-CSGMembershipStatus": {"type": "ProtocolIE-ID", "value": 146},
            "id-Cause": {"type": "ProtocolIE-ID", "value": 2},
            "id-CellAccessMode": {"type": "ProtocolIE-ID", "value": 145},
            "id-CellIdentifierAndCELevelForCECapableUEs": {
                "type": "ProtocolIE-ID",
                "value": 212,
            },
            "id-CellTrafficTrace": {"type": "ProcedureCode", "value": 42},
            "id-ConcurrentWarningMessageIndicator": {
                "type": "ProtocolIE-ID",
                "value": 142,
            },
            "id-ConnectionEstablishmentIndication": {
                "type": "ProcedureCode",
                "value": 54,
            },
            "id-Correlation-ID": {"type": "ProtocolIE-ID", "value": 156},
            "id-Coverage-Level": {"type": "ProtocolIE-ID", "value": 250},
            "id-CriticalityDiagnostics": {"type": "ProtocolIE-ID", "value": 58},
            "id-DCN-ID": {"type": "ProtocolIE-ID", "value": 246},
            "id-DL-CP-SecurityInformation": {"type": "ProtocolIE-ID", "value": 253},
            "id-DLCOUNTValueExtended": {"type": "ProtocolIE-ID", "value": 180},
            "id-DLCOUNTValuePDCP-SNlength18": {"type": "ProtocolIE-ID", "value": 218},
            "id-DLNASPDUDeliveryAckRequest": {"type": "ProtocolIE-ID", "value": 249},
            "id-Data-Forwarding-Not-Possible": {"type": "ProtocolIE-ID", "value": 143},
            "id-DataCodingScheme": {"type": "ProtocolIE-ID", "value": 118},
            "id-DeactivateTrace": {"type": "ProcedureCode", "value": 26},
            "id-DefaultPagingDRX": {"type": "ProtocolIE-ID", "value": 137},
            "id-Direct-Forwarding-Path-Availability": {
                "type": "ProtocolIE-ID",
                "value": 79,
            },
            "id-DownlinkS1cdma2000tunnelling": {"type": "ProcedureCode", "value": 19},
            "id-E-RABAdmittedItem": {"type": "ProtocolIE-ID", "value": 20},
            "id-E-RABAdmittedList": {"type": "ProtocolIE-ID", "value": 18},
            "id-E-RABDataForwardingItem": {"type": "ProtocolIE-ID", "value": 14},
            "id-E-RABFailedToBeReleasedList": {"type": "ProtocolIE-ID", "value": 103},
            "id-E-RABFailedToModifyList": {"type": "ProtocolIE-ID", "value": 32},
            "id-E-RABFailedToModifyListBearerModConf": {
                "type": "ProtocolIE-ID",
                "value": 205,
            },
            "id-E-RABFailedToReleaseList": {"type": "ProtocolIE-ID", "value": 34},
            "id-E-RABFailedToResumeItemResumeReq": {
                "type": "ProtocolIE-ID",
                "value": 236,
            },
            "id-E-RABFailedToResumeItemResumeRes": {
                "type": "ProtocolIE-ID",
                "value": 238,
            },
            "id-E-RABFailedToResumeListResumeReq": {
                "type": "ProtocolIE-ID",
                "value": 235,
            },
            "id-E-RABFailedToResumeListResumeRes": {
                "type": "ProtocolIE-ID",
                "value": 237,
            },
            "id-E-RABFailedToSetupListBearerSURes": {
                "type": "ProtocolIE-ID",
                "value": 29,
            },
            "id-E-RABFailedToSetupListCtxtSURes": {
                "type": "ProtocolIE-ID",
                "value": 48,
            },
            "id-E-RABFailedToSetupListHOReqAck": {"type": "ProtocolIE-ID", "value": 19},
            "id-E-RABFailedtoSetupItemHOReqAck": {"type": "ProtocolIE-ID", "value": 21},
            "id-E-RABInformationListItem": {"type": "ProtocolIE-ID", "value": 78},
            "id-E-RABItem": {"type": "ProtocolIE-ID", "value": 35},
            "id-E-RABModificationIndication": {"type": "ProcedureCode", "value": 50},
            "id-E-RABModify": {"type": "ProcedureCode", "value": 6},
            "id-E-RABModifyItemBearerModConf": {"type": "ProtocolIE-ID", "value": 204},
            "id-E-RABModifyItemBearerModRes": {"type": "ProtocolIE-ID", "value": 37},
            "id-E-RABModifyListBearerModConf": {"type": "ProtocolIE-ID", "value": 203},
            "id-E-RABModifyListBearerModRes": {"type": "ProtocolIE-ID", "value": 31},
            "id-E-RABNotToBeModifiedItemBearerModInd": {
                "type": "ProtocolIE-ID",
                "value": 202,
            },
            "id-E-RABNotToBeModifiedListBearerModInd": {
                "type": "ProtocolIE-ID",
                "value": 201,
            },
            "id-E-RABRelease": {"type": "ProcedureCode", "value": 7},
            "id-E-RABReleaseIndication": {"type": "ProcedureCode", "value": 8},
            "id-E-RABReleaseItem": {"type": "ProtocolIE-ID", "value": 38},
            "id-E-RABReleaseItemBearerRelComp": {"type": "ProtocolIE-ID", "value": 15},
            "id-E-RABReleaseItemHOCmd": {"type": "ProtocolIE-ID", "value": 49},
            "id-E-RABReleaseListBearerRelComp": {"type": "ProtocolIE-ID", "value": 69},
            "id-E-RABReleasedList": {"type": "ProtocolIE-ID", "value": 110},
            "id-E-RABSetup": {"type": "ProcedureCode", "value": 5},
            "id-E-RABSetupItemBearerSURes": {"type": "ProtocolIE-ID", "value": 39},
            "id-E-RABSetupItemCtxtSURes": {"type": "ProtocolIE-ID", "value": 50},
            "id-E-RABSetupListBearerSURes": {"type": "ProtocolIE-ID", "value": 28},
            "id-E-RABSetupListCtxtSURes": {"type": "ProtocolIE-ID", "value": 51},
            "id-E-RABSubjecttoDataForwardingList": {
                "type": "ProtocolIE-ID",
                "value": 12,
            },
            "id-E-RABToBeModifiedItemBearerModInd": {
                "type": "ProtocolIE-ID",
                "value": 200,
            },
            "id-E-RABToBeModifiedItemBearerModReq": {
                "type": "ProtocolIE-ID",
                "value": 36,
            },
            "id-E-RABToBeModifiedListBearerModInd": {
                "type": "ProtocolIE-ID",
                "value": 199,
            },
            "id-E-RABToBeModifiedListBearerModReq": {
                "type": "ProtocolIE-ID",
                "value": 30,
            },
            "id-E-RABToBeReleasedList": {"type": "ProtocolIE-ID", "value": 33},
            "id-E-RABToBeReleasedListBearerModConf": {
                "type": "ProtocolIE-ID",
                "value": 210,
            },
            "id-E-RABToBeSetupItemBearerSUReq": {"type": "ProtocolIE-ID", "value": 17},
            "id-E-RABToBeSetupItemCtxtSUReq": {"type": "ProtocolIE-ID", "value": 52},
            "id-E-RABToBeSetupItemHOReq": {"type": "ProtocolIE-ID", "value": 27},
            "id-E-RABToBeSetupListBearerSUReq": {"type": "ProtocolIE-ID", "value": 16},
            "id-E-RABToBeSetupListCtxtSUReq": {"type": "ProtocolIE-ID", "value": 24},
            "id-E-RABToBeSetupListHOReq": {"type": "ProtocolIE-ID", "value": 53},
            "id-E-RABToBeSwitchedDLItem": {"type": "ProtocolIE-ID", "value": 23},
            "id-E-RABToBeSwitchedDLList": {"type": "ProtocolIE-ID", "value": 22},
            "id-E-RABToBeSwitchedULItem": {"type": "ProtocolIE-ID", "value": 94},
            "id-E-RABToBeSwitchedULList": {"type": "ProtocolIE-ID", "value": 95},
            "id-E-RABtoReleaseListHOCmd": {"type": "ProtocolIE-ID", "value": 13},
            "id-E-UTRAN-Trace-ID": {"type": "ProtocolIE-ID", "value": 86},
            "id-ECGIListForRestart": {"type": "ProtocolIE-ID", "value": 182},
            "id-ENBConfigurationUpdate": {"type": "ProcedureCode", "value": 29},
            "id-EUTRAN-CGI": {"type": "ProtocolIE-ID", "value": 100},
            "id-EUTRANRoundTripDelayEstimationInfo": {
                "type": "ProtocolIE-ID",
                "value": 140,
            },
            "id-EmergencyAreaIDListForRestart": {"type": "ProtocolIE-ID", "value": 190},
            "id-EnhancedCoverageRestricted": {"type": "ProtocolIE-ID", "value": 251},
            "id-ErrorIndication": {"type": "ProcedureCode", "value": 15},
            "id-ExpectedUEBehaviour": {"type": "ProtocolIE-ID", "value": 196},
            "id-ExtendedRepetitionPeriod": {"type": "ProtocolIE-ID", "value": 144},
            "id-GERANtoLTEHOInformationRes": {"type": "ProtocolIE-ID", "value": 55},
            "id-GUMMEI-ID": {"type": "ProtocolIE-ID", "value": 75},
            "id-GUMMEIList": {"type": "ProtocolIE-ID", "value": 154},
            "id-GUMMEIType": {"type": "ProtocolIE-ID", "value": 170},
            "id-GW-TransportLayerAddress": {"type": "ProtocolIE-ID", "value": 155},
            "id-GWContextReleaseIndication": {"type": "ProtocolIE-ID", "value": 164},
            "id-Global-ENB-ID": {"type": "ProtocolIE-ID", "value": 59},
            "id-HO-Cause": {"type": "ProtocolIE-ID", "value": 168},
            "id-HandoverCancel": {"type": "ProcedureCode", "value": 4},
            "id-HandoverNotification": {"type": "ProcedureCode", "value": 2},
            "id-HandoverPreparation": {"type": "ProcedureCode", "value": 0},
            "id-HandoverResourceAllocation": {"type": "ProcedureCode", "value": 1},
            "id-HandoverRestrictionList": {"type": "ProtocolIE-ID", "value": 41},
            "id-HandoverType": {"type": "ProtocolIE-ID", "value": 1},
            "id-InformationOnRecommendedCellsAndENBsForPaging": {
                "type": "ProtocolIE-ID",
                "value": 213,
            },
            "id-InitialContextSetup": {"type": "ProcedureCode", "value": 9},
            "id-Inter-SystemInformationTransferTypeEDT": {
                "type": "ProtocolIE-ID",
                "value": 121,
            },
            "id-Inter-SystemInformationTransferTypeMDT": {
                "type": "ProtocolIE-ID",
                "value": 122,
            },
            "id-Kill": {"type": "ProcedureCode", "value": 43},
            "id-KillAllWarningMessages": {"type": "ProtocolIE-ID", "value": 191},
            "id-LHN-ID": {"type": "ProtocolIE-ID", "value": 186},
            "id-LPPa-PDU": {"type": "ProtocolIE-ID", "value": 147},
            "id-LocationReport": {"type": "ProcedureCode", "value": 33},
            "id-LocationReportingControl": {"type": "ProcedureCode", "value": 31},
            "id-LocationReportingFailureIndication": {
                "type": "ProcedureCode",
                "value": 32,
            },
            "id-LoggedMBSFNMDT": {"type": "ProtocolIE-ID", "value": 197},
            "id-M3Configuration": {"type": "ProtocolIE-ID", "value": 171},
            "id-M4Configuration": {"type": "ProtocolIE-ID", "value": 172},
            "id-M5Configuration": {"type": "ProtocolIE-ID", "value": 173},
            "id-M6Configuration": {"type": "ProtocolIE-ID", "value": 220},
            "id-M7Configuration": {"type": "ProtocolIE-ID", "value": 221},
            "id-MDT-Location-Info": {"type": "ProtocolIE-ID", "value": 174},
            "id-MDTConfiguration": {"type": "ProtocolIE-ID", "value": 162},
            "id-MME-Group-ID": {"type": "ProtocolIE-ID", "value": 223},
            "id-MME-UE-S1AP-ID": {"type": "ProtocolIE-ID", "value": 0},
            "id-MME-UE-S1AP-ID-2": {"type": "ProtocolIE-ID", "value": 158},
            "id-MMECPRelocationIndication": {"type": "ProcedureCode", "value": 61},
            "id-MMEConfigurationTransfer": {"type": "ProcedureCode", "value": 41},
            "id-MMEConfigurationUpdate": {"type": "ProcedureCode", "value": 30},
            "id-MMEDirectInformationTransfer": {"type": "ProcedureCode", "value": 38},
            "id-MMERelaySupportIndicator": {"type": "ProtocolIE-ID", "value": 163},
            "id-MMEStatusTransfer": {"type": "ProcedureCode", "value": 25},
            "id-MMEname": {"type": "ProtocolIE-ID", "value": 61},
            "id-MSClassmark2": {"type": "ProtocolIE-ID", "value": 132},
            "id-MSClassmark3": {"type": "ProtocolIE-ID", "value": 133},
            "id-ManagementBasedMDTAllowed": {"type": "ProtocolIE-ID", "value": 165},
            "id-ManagementBasedMDTPLMNList": {"type": "ProtocolIE-ID", "value": 177},
            "id-Masked-IMEISV": {"type": "ProtocolIE-ID", "value": 192},
            "id-MessageIdentifier": {"type": "ProtocolIE-ID", "value": 111},
            "id-MobilityInformation": {"type": "ProtocolIE-ID", "value": 175},
            "id-Muting-Availability-Indication": {
                "type": "ProtocolIE-ID",
                "value": 207,
            },
            "id-Muting-Pattern-Information": {"type": "ProtocolIE-ID", "value": 208},
            "id-NAS-DownlinkCount": {"type": "ProtocolIE-ID", "value": 126},
            "id-NAS-PDU": {"type": "ProtocolIE-ID", "value": 26},
            "id-NASDeliveryIndication": {"type": "ProcedureCode", "value": 57},
            "id-NASNonDeliveryIndication": {"type": "ProcedureCode", "value": 16},
            "id-NASSecurityParametersfromE-UTRAN": {
                "type": "ProtocolIE-ID",
                "value": 135,
            },
            "id-NASSecurityParameterstoE-UTRAN": {
                "type": "ProtocolIE-ID",
                "value": 136,
            },
            "id-NB-IoT-DefaultPagingDRX": {"type": "ProtocolIE-ID", "value": 234},
            "id-NB-IoT-Paging-eDRXInformation": {"type": "ProtocolIE-ID", "value": 239},
            "id-NB-IoT-UEIdentityIndexValue": {"type": "ProtocolIE-ID", "value": 244},
            "id-NumberofBroadcastRequest": {"type": "ProtocolIE-ID", "value": 115},
            "id-OverloadResponse": {"type": "ProtocolIE-ID", "value": 101},
            "id-OverloadStart": {"type": "ProcedureCode", "value": 34},
            "id-OverloadStop": {"type": "ProcedureCode", "value": 35},
            "id-PS-ServiceNotAvailable": {"type": "ProtocolIE-ID", "value": 150},
            "id-PWSFailureIndication": {"type": "ProcedureCode", "value": 51},
            "id-PWSRestartIndication": {"type": "ProcedureCode", "value": 49},
            "id-PWSfailedECGIList": {"type": "ProtocolIE-ID", "value": 222},
            "id-Paging": {"type": "ProcedureCode", "value": 10},
            "id-Paging-eDRXInformation": {"type": "ProtocolIE-ID", "value": 227},
            "id-PagingPriority": {"type": "ProtocolIE-ID", "value": 151},
            "id-PathSwitchRequest": {"type": "ProcedureCode", "value": 3},
            "id-PrivacyIndicator": {"type": "ProtocolIE-ID", "value": 166},
            "id-PrivateMessage": {"type": "ProcedureCode", "value": 39},
            "id-ProSeAuthorized": {"type": "ProtocolIE-ID", "value": 195},
            "id-ProSeUEtoNetworkRelaying": {"type": "ProtocolIE-ID", "value": 216},
            "id-RAT-Type": {"type": "ProtocolIE-ID", "value": 232},
            "id-RRC-Establishment-Cause": {"type": "ProtocolIE-ID", "value": 134},
            "id-RRC-Resume-Cause": {"type": "ProtocolIE-ID", "value": 245},
            "id-ReceiveStatusOfULPDCPSDUsExtended": {
                "type": "ProtocolIE-ID",
                "value": 181,
            },
            "id-ReceiveStatusOfULPDCPSDUsPDCP-SNlength18": {
                "type": "ProtocolIE-ID",
                "value": 219,
            },
            "id-RecommendedCellItem": {"type": "ProtocolIE-ID", "value": 214},
            "id-RecommendedENBItem": {"type": "ProtocolIE-ID", "value": 215},
            "id-RegisteredLAI": {"type": "ProtocolIE-ID", "value": 159},
            "id-RelativeMMECapacity": {"type": "ProtocolIE-ID", "value": 87},
            "id-RelayNode-Indicator": {"type": "ProtocolIE-ID", "value": 160},
            "id-RepetitionPeriod": {"type": "ProtocolIE-ID", "value": 114},
            "id-RequestType": {"type": "ProtocolIE-ID", "value": 98},
            "id-RerouteNASRequest": {"type": "ProcedureCode", "value": 52},
            "id-Reset": {"type": "ProcedureCode", "value": 14},
            "id-ResetType": {"type": "ProtocolIE-ID", "value": 92},
            "id-RetrieveUEInformation": {"type": "ProcedureCode", "value": 58},
            "id-Routing-ID": {"type": "ProtocolIE-ID", "value": 148},
            "id-S-TMSI": {"type": "ProtocolIE-ID", "value": 96},
            "id-S1-Message": {"type": "ProtocolIE-ID", "value": 225},
            "id-S1Setup": {"type": "ProcedureCode", "value": 17},
            "id-SIPTO-Correlation-ID": {"type": "ProtocolIE-ID", "value": 183},
            "id-SIPTO-L-GW-TransportLayerAddress": {
                "type": "ProtocolIE-ID",
                "value": 184,
            },
            "id-SON-Information-Report": {"type": "ProtocolIE-ID", "value": 206},
            "id-SONConfigurationTransferECT": {"type": "ProtocolIE-ID", "value": 129},
            "id-SONConfigurationTransferMCT": {"type": "ProtocolIE-ID", "value": 130},
            "id-SRVCCHOIndication": {"type": "ProtocolIE-ID", "value": 125},
            "id-SRVCCOperationNotPossible": {"type": "ProtocolIE-ID", "value": 243},
            "id-SRVCCOperationPossible": {"type": "ProtocolIE-ID", "value": 124},
            "id-SecurityContext": {"type": "ProtocolIE-ID", "value": 40},
            "id-SecurityKey": {"type": "ProtocolIE-ID", "value": 73},
            "id-SerialNumber": {"type": "ProtocolIE-ID", "value": 112},
            "id-ServedDCNs": {"type": "ProtocolIE-ID", "value": 247},
            "id-ServedGUMMEIs": {"type": "ProtocolIE-ID", "value": 105},
            "id-ServedPLMNs": {"type": "ProtocolIE-ID", "value": 63},
            "id-SignallingBasedMDTPLMNList": {"type": "ProtocolIE-ID", "value": 178},
            "id-Source-ToTarget-TransparentContainer": {
                "type": "ProtocolIE-ID",
                "value": 104,
            },
            "id-Source-ToTarget-TransparentContainer-Secondary": {
                "type": "ProtocolIE-ID",
                "value": 138,
            },
            "id-SourceID": {"type": "ProtocolIE-ID", "value": 3},
            "id-SourceMME-GUMMEI": {"type": "ProtocolIE-ID", "value": 157},
            "id-SourceMME-UE-S1AP-ID": {"type": "ProtocolIE-ID", "value": 88},
            "id-SubscriberProfileIDforRFP": {"type": "ProtocolIE-ID", "value": 106},
            "id-SupportedTAs": {"type": "ProtocolIE-ID", "value": 64},
            "id-Synchronisation-Information": {"type": "ProtocolIE-ID", "value": 209},
            "id-TAI": {"type": "ProtocolIE-ID", "value": 67},
            "id-TAIItem": {"type": "ProtocolIE-ID", "value": 47},
            "id-TAIList": {"type": "ProtocolIE-ID", "value": 46},
            "id-TAIListForRestart": {"type": "ProtocolIE-ID", "value": 188},
            "id-Target-ToSource-TransparentContainer": {
                "type": "ProtocolIE-ID",
                "value": 123,
            },
            "id-Target-ToSource-TransparentContainer-Secondary": {
                "type": "ProtocolIE-ID",
                "value": 139,
            },
            "id-TargetID": {"type": "ProtocolIE-ID", "value": 4},
            "id-Time-Synchronisation-Info": {"type": "ProtocolIE-ID", "value": 149},
            "id-Time-UE-StayedInCell-EnhancedGranularity": {
                "type": "ProtocolIE-ID",
                "value": 167,
            },
            "id-TimeToWait": {"type": "ProtocolIE-ID", "value": 65},
            "id-TraceActivation": {"type": "ProtocolIE-ID", "value": 25},
            "id-TraceCollectionEntityIPAddress": {
                "type": "ProtocolIE-ID",
                "value": 131,
            },
            "id-TraceFailureIndication": {"type": "ProcedureCode", "value": 28},
            "id-TraceStart": {"type": "ProcedureCode", "value": 27},
            "id-TrafficLoadReductionIndication": {
                "type": "ProtocolIE-ID",
                "value": 161,
            },
            "id-TransportInformation": {"type": "ProtocolIE-ID", "value": 185},
            "id-Tunnel-Information-for-BBF": {"type": "ProtocolIE-ID", "value": 176},
            "id-UE-Level-QoS-Parameters": {"type": "ProtocolIE-ID", "value": 252},
            "id-UE-RetentionInformation": {"type": "ProtocolIE-ID", "value": 228},
            "id-UE-S1AP-IDs": {"type": "ProtocolIE-ID", "value": 99},
            "id-UE-Usage-Type": {"type": "ProtocolIE-ID", "value": 230},
            "id-UE-associatedLogicalS1-ConnectionItem": {
                "type": "ProtocolIE-ID",
                "value": 91,
            },
            "id-UE-associatedLogicalS1-ConnectionListResAck": {
                "type": "ProtocolIE-ID",
                "value": 93,
            },
            "id-UECapabilityInfoIndication": {"type": "ProcedureCode", "value": 22},
            "id-UEContextModification": {"type": "ProcedureCode", "value": 21},
            "id-UEContextModificationIndication": {
                "type": "ProcedureCode",
                "value": 53,
            },
            "id-UEContextRelease": {"type": "ProcedureCode", "value": 23},
            "id-UEContextReleaseRequest": {"type": "ProcedureCode", "value": 18},
            "id-UEContextResume": {"type": "ProcedureCode", "value": 56},
            "id-UEContextSuspend": {"type": "ProcedureCode", "value": 55},
            "id-UEIdentityIndexValue": {"type": "ProtocolIE-ID", "value": 80},
            "id-UEInformationTransfer": {"type": "ProcedureCode", "value": 59},
            "id-UEPagingID": {"type": "ProtocolIE-ID", "value": 43},
            "id-UERadioCapability": {"type": "ProtocolIE-ID", "value": 74},
            "id-UERadioCapabilityForPaging": {"type": "ProtocolIE-ID", "value": 198},
            "id-UERadioCapabilityMatch": {"type": "ProcedureCode", "value": 48},
            "id-UESecurityCapabilities": {"type": "ProtocolIE-ID", "value": 107},
            "id-UESidelinkAggregateMaximumBitrate": {
                "type": "ProtocolIE-ID",
                "value": 248,
            },
            "id-UEUserPlaneCIoTSupportIndicator": {
                "type": "ProtocolIE-ID",
                "value": 241,
            },
            "id-UL-CP-SecurityInformation": {"type": "ProtocolIE-ID", "value": 254},
            "id-ULCOUNTValueExtended": {"type": "ProtocolIE-ID", "value": 179},
            "id-ULCOUNTValuePDCP-SNlength18": {"type": "ProtocolIE-ID", "value": 217},
            "id-UTRANtoLTEHOInformationRes": {"type": "ProtocolIE-ID", "value": 57},
            "id-UplinkS1cdma2000tunnelling": {"type": "ProcedureCode", "value": 20},
            "id-UserLocationInformation": {"type": "ProtocolIE-ID", "value": 189},
            "id-V2XServicesAuthorized": {"type": "ProtocolIE-ID", "value": 240},
            "id-VoiceSupportMatchIndicator": {"type": "ProtocolIE-ID", "value": 169},
            "id-WarningAreaList": {"type": "ProtocolIE-ID", "value": 113},
            "id-WarningMessageContents": {"type": "ProtocolIE-ID", "value": 119},
            "id-WarningSecurityInfo": {"type": "ProtocolIE-ID", "value": 117},
            "id-WarningType": {"type": "ProtocolIE-ID", "value": 116},
            "id-WriteReplaceWarning": {"type": "ProcedureCode", "value": 36},
            "id-cdma2000HORequiredIndication": {"type": "ProtocolIE-ID", "value": 84},
            "id-cdma2000HOStatus": {"type": "ProtocolIE-ID", "value": 83},
            "id-cdma2000OneXRAND": {"type": "ProtocolIE-ID", "value": 97},
            "id-cdma2000OneXSRVCCInfo": {"type": "ProtocolIE-ID", "value": 102},
            "id-cdma2000PDU": {"type": "ProtocolIE-ID", "value": 70},
            "id-cdma2000RATType": {"type": "ProtocolIE-ID", "value": 71},
            "id-cdma2000SectorID": {"type": "ProtocolIE-ID", "value": 72},
            "id-downlinkNASTransport": {"type": "ProcedureCode", "value": 11},
            "id-downlinkNonUEAssociatedLPPaTransport": {
                "type": "ProcedureCode",
                "value": 46,
            },
            "id-downlinkUEAssociatedLPPaTransport": {
                "type": "ProcedureCode",
                "value": 44,
            },
            "id-eNB-StatusTransfer-TransparentContainer": {
                "type": "ProtocolIE-ID",
                "value": 90,
            },
            "id-eNB-UE-S1AP-ID": {"type": "ProtocolIE-ID", "value": 8},
            "id-eNBCPRelocationIndication": {"type": "ProcedureCode", "value": 60},
            "id-eNBConfigurationTransfer": {"type": "ProcedureCode", "value": 40},
            "id-eNBDirectInformationTransfer": {"type": "ProcedureCode", "value": 37},
            "id-eNBIndirectX2TransportLayerAddresses": {
                "type": "ProtocolIE-ID",
                "value": 193,
            },
            "id-eNBStatusTransfer": {"type": "ProcedureCode", "value": 24},
            "id-eNBX2ExtendedTransportLayerAddresses": {
                "type": "ProtocolIE-ID",
                "value": 153,
            },
            "id-eNBname": {"type": "ProtocolIE-ID", "value": 60},
            "id-extended-UEIdentityIndexValue": {"type": "ProtocolIE-ID", "value": 231},
            "id-initialUEMessage": {"type": "ProcedureCode", "value": 12},
            "id-pagingDRX": {"type": "ProtocolIE-ID", "value": 44},
            "id-uE-HistoryInformationFromTheUE": {
                "type": "ProtocolIE-ID",
                "value": 194,
            },
            "id-uEaggregateMaximumBitrate": {"type": "ProtocolIE-ID", "value": 66},
            "id-uplinkNASTransport": {"type": "ProcedureCode", "value": 13},
            "id-uplinkNonUEAssociatedLPPaTransport": {
                "type": "ProcedureCode",
                "value": 47,
            },
            "id-uplinkUEAssociatedLPPaTransport": {
                "type": "ProcedureCode",
                "value": 45,
            },
            "id-x2TNLConfigurationInfo": {"type": "ProtocolIE-ID", "value": 152},
            "maxEARFCN": {"type": "INTEGER", "value": 262143},
            "maxPrivateIEs": {"type": "INTEGER", "value": 65535},
            "maxProtocolExtensions": {"type": "INTEGER", "value": 65535},
            "maxProtocolIEs": {"type": "INTEGER", "value": 65535},
            "maxnoofBPLMNs": {"type": "INTEGER", "value": 6},
            "maxnoofCSGs": {"type": "INTEGER", "value": 256},
            "maxnoofCellID": {"type": "INTEGER", "value": 65535},
            "maxnoofCellIDforMDT": {"type": "INTEGER", "value": 32},
            "maxnoofCellinEAI": {"type": "INTEGER", "value": 65535},
            "maxnoofCellinTAI": {"type": "INTEGER", "value": 65535},
            "maxnoofCells": {"type": "INTEGER", "value": 16},
            "maxnoofCellsforRestart": {"type": "INTEGER", "value": 256},
            "maxnoofCellsineNB": {"type": "INTEGER", "value": 256},
            "maxnoofDCNs": {"type": "INTEGER", "value": 32},
            "maxnoofE-RABs": {"type": "INTEGER", "value": 256},
            "maxnoofEPLMNs": {"type": "INTEGER", "value": 15},
            "maxnoofEPLMNsPlusOne": {"type": "INTEGER", "value": 16},
            "maxnoofEmergencyAreaID": {"type": "INTEGER", "value": 65535},
            "maxnoofErrors": {"type": "INTEGER", "value": 256},
            "maxnoofForbLACs": {"type": "INTEGER", "value": 4096},
            "maxnoofForbTACs": {"type": "INTEGER", "value": 4096},
            "maxnoofGroupIDs": {"type": "INTEGER", "value": 65535},
            "maxnoofIndividualS1ConnectionsToReset": {"type": "INTEGER", "value": 256},
            "maxnoofMBSFNAreaMDT": {"type": "INTEGER", "value": 8},
            "maxnoofMDTPLMNs": {"type": "INTEGER", "value": 16},
            "maxnoofMMECs": {"type": "INTEGER", "value": 256},
            "maxnoofPLMNsPerMME": {"type": "INTEGER", "value": 32},
            "maxnoofRATs": {"type": "INTEGER", "value": 8},
            "maxnoofRecommendedCells": {"type": "INTEGER", "value": 16},
            "maxnoofRecommendedENBs": {"type": "INTEGER", "value": 16},
            "maxnoofRestartEmergencyAreaIDs": {"type": "INTEGER", "value": 256},
            "maxnoofRestartTAIs": {"type": "INTEGER", "value": 2048},
            "maxnoofTACs": {"type": "INTEGER", "value": 256},
            "maxnoofTAIforWarning": {"type": "INTEGER", "value": 65535},
            "maxnoofTAIs": {"type": "INTEGER", "value": 256},
            "maxnoofTAforMDT": {"type": "INTEGER", "value": 8},
            "maxnoofeNBX2ExtTLAs": {"type": "INTEGER", "value": 16},
            "maxnoofeNBX2GTPTLAs": {"type": "INTEGER", "value": 16},
            "maxnoofeNBX2TLAs": {"type": "INTEGER", "value": 2},
        },
    },
    "S1AP-Containers": {
        "extensibility-implied": False,
        "imports": {
            "S1AP-CommonDataTypes": [
                "Criticality",
                "Presence",
                "PrivateIE-ID",
                "ProtocolExtensionID",
                "ProtocolIE-ID",
            ],
            "S1AP-Constants": [
                "maxPrivateIEs",
                "maxProtocolExtensions",
                "maxProtocolIEs",
            ],
        },
        "object-classes": {
            "S1AP-PRIVATE-IES": {
                "members": [
                    {"name": "&id", "type": "PrivateIE-ID"},
                    {"name": "&criticality", "type": "Criticality"},
                    {"name": "&Value", "type": "OpenType"},
                    {"name": "&presence", "type": "Presence"},
                ],
                "syntax": [
                    "ID",
                    "&id",
                    "CRITICALITY",
                    "&criticality",
                    "TYPE",
                    "&Value",
                    "PRESENCE",
                    "&presence",
                ],
            },
            "S1AP-PROTOCOL-EXTENSION": {
                "members": [
                    {"name": "&id", "type": "ProtocolExtensionID"},
                    {"name": "&criticality", "type": "Criticality"},
                    {"name": "&Extension", "type": "OpenType"},
                    {"name": "&presence", "type": "Presence"},
                ],
                "syntax": [
                    "ID",
                    "&id",
                    "CRITICALITY",
                    "&criticality",
                    "EXTENSION",
                    "&Extension",
                    "PRESENCE",
                    "&presence",
                ],
            },
            "S1AP-PROTOCOL-IES": {
                "members": [
                    {"name": "&id", "type": "ProtocolIE-ID"},
                    {"name": "&criticality", "type": "Criticality"},
                    {"name": "&Value", "type": "OpenType"},
                    {"name": "&presence", "type": "Presence"},
                ],
                "syntax": [
                    "ID",
                    "&id",
                    "CRITICALITY",
                    "&criticality",
                    "TYPE",
                    "&Value",
                    "PRESENCE",
                    "&presence",
                ],
            },
            "S1AP-PROTOCOL-IES-PAIR": {
                "members": [
                    {"name": "&id", "type": "ProtocolIE-ID"},
                    {"name": "&firstCriticality", "type": "Criticality"},
                    {"name": "&FirstValue", "type": "OpenType"},
                    {"name": "&secondCriticality", "type": "Criticality"},
                    {"name": "&SecondValue", "type": "OpenType"},
                    {"name": "&presence", "type": "Presence"},
                ],
                "syntax": [
                    "ID",
                    "&id",
                    "FIRST",
                    "CRITICALITY",
                    "&firstCriticality",
                    "FIRST",
                    "TYPE",
                    "&FirstValue",
                    "SECOND",
                    "CRITICALITY",
                    "&secondCriticality",
                    "SECOND",
                    "TYPE",
                    "&SecondValue",
                    "PRESENCE",
                    "&presence",
                ],
            },
        },
        "object-sets": {},
        "tags": "AUTOMATIC",
        "types": {
            "PrivateIE-Container": {
                "element": {
                    "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                    "type": "PrivateIE-Field",
                },
                "parameter-governors": [{"type": "S1AP-PRIVATE-IES"}],
                "parameters": ["IEsSetParam"],
                "size": [(1, "maxPrivateIEs")],
                "type": "SEQUENCE OF",
            },
            "PrivateIE-Field": {
                "members": [
                    {
                        "name": "id",
                        "table": {"type": "IEsSetParam"},
                        "type": "S1AP-PRIVATE-IES.&id",
                    },
                    {
                        "name": "criticality",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PRIVATE-IES.&criticality",
                    },
                    {
                        "name": "value",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PRIVATE-IES.&Value",
                    },
                ],
                "parameter-governors": [{"type": "S1AP-PRIVATE-IES"}],
                "parameters": ["IEsSetParam"],
                "type": "SEQUENCE",
            },
            "ProtocolExtensionContainer": {
                "element": {
                    "actual-parameters": [{"object-set": ["ExtensionSetParam"]}],
                    "type": "ProtocolExtensionField",
                },
                "parameter-governors": [{"type": "S1AP-PROTOCOL-EXTENSION"}],
                "parameters": ["ExtensionSetParam"],
                "size": [(1, "maxProtocolExtensions")],
                "type": "SEQUENCE OF",
            },
            "ProtocolExtensionField": {
                "members": [
                    {
                        "name": "id",
                        "table": {"type": "ExtensionSetParam"},
                        "type": "S1AP-PROTOCOL-EXTENSION.&id",
                    },
                    {
                        "name": "criticality",
                        "table": ["ExtensionSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-EXTENSION.&criticality",
                    },
                    {
                        "name": "extensionValue",
                        "table": ["ExtensionSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-EXTENSION.&Extension",
                    },
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-EXTENSION"}],
                "parameters": ["ExtensionSetParam"],
                "type": "SEQUENCE",
            },
            "ProtocolIE-Container": {
                "element": {
                    "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                    "type": "ProtocolIE-Field",
                },
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES"}],
                "parameters": ["IEsSetParam"],
                "size": [(0, "maxProtocolIEs")],
                "type": "SEQUENCE OF",
            },
            "ProtocolIE-ContainerList": {
                "element": {
                    "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "parameter-governors": [
                    {"type": "INTEGER"},
                    {"type": "INTEGER"},
                    {"type": "S1AP-PROTOCOL-IES"},
                ],
                "parameters": ["lowerBound", "upperBound", "IEsSetParam"],
                "size": [("lowerBound", "upperBound")],
                "type": "SEQUENCE OF",
            },
            "ProtocolIE-ContainerPair": {
                "element": {
                    "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                    "type": "ProtocolIE-FieldPair",
                },
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES-PAIR"}],
                "parameters": ["IEsSetParam"],
                "size": [(0, "maxProtocolIEs")],
                "type": "SEQUENCE OF",
            },
            "ProtocolIE-ContainerPairList": {
                "element": {
                    "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                    "type": "ProtocolIE-ContainerPair",
                },
                "parameter-governors": [
                    {"type": "INTEGER"},
                    {"type": "INTEGER"},
                    {"type": "S1AP-PROTOCOL-IES-PAIR"},
                ],
                "parameters": ["lowerBound", "upperBound", "IEsSetParam"],
                "size": [("lowerBound", "upperBound")],
                "type": "SEQUENCE OF",
            },
            "ProtocolIE-Field": {
                "members": [
                    {
                        "name": "id",
                        "table": {"type": "IEsSetParam"},
                        "type": "S1AP-PROTOCOL-IES.&id",
                    },
                    {
                        "name": "criticality",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES.&criticality",
                    },
                    {
                        "name": "value",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES.&Value",
                    },
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES"}],
                "parameters": ["IEsSetParam"],
                "type": "SEQUENCE",
            },
            "ProtocolIE-FieldPair": {
                "members": [
                    {
                        "name": "id",
                        "table": {"type": "IEsSetParam"},
                        "type": "S1AP-PROTOCOL-IES-PAIR.&id",
                    },
                    {
                        "name": "firstCriticality",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES-PAIR.&firstCriticality",
                    },
                    {
                        "name": "firstValue",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES-PAIR.&FirstValue",
                    },
                    {
                        "name": "secondCriticality",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES-PAIR.&secondCriticality",
                    },
                    {
                        "name": "secondValue",
                        "table": ["IEsSetParam", ["id"]],
                        "type": "S1AP-PROTOCOL-IES-PAIR.&SecondValue",
                    },
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES-PAIR"}],
                "parameters": ["IEsSetParam"],
                "type": "SEQUENCE",
            },
            "ProtocolIE-SingleContainer": {
                "actual-parameters": [{"object-set": ["IEsSetParam"]}],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES"}],
                "parameters": ["IEsSetParam"],
                "type": "ProtocolIE-Field",
            },
        },
        "values": {},
    },
    "S1AP-IEs": {
        "extensibility-implied": False,
        "imports": {
            "S1AP-CommonDataTypes": [
                "Criticality",
                "ProcedureCode",
                "ProtocolIE-ID",
                "TriggeringMessage",
            ],
            "S1AP-Constants": [
                "id-Bearers-SubjectToStatusTransfer-Item",
                "id-DLCOUNTValueExtended",
                "id-DLCOUNTValuePDCP-SNlength18",
                "id-E-RABInformationListItem",
                "id-E-RABItem",
                "id-HO-Cause",
                "id-LoggedMBSFNMDT",
                "id-M3Configuration",
                "id-M4Configuration",
                "id-M5Configuration",
                "id-M6Configuration",
                "id-M7Configuration",
                "id-MDT-Location-Info",
                "id-MDTConfiguration",
                "id-MobilityInformation",
                "id-Muting-Availability-Indication",
                "id-Muting-Pattern-Information",
                "id-ProSeUEtoNetworkRelaying",
                "id-RAT-Type",
                "id-ReceiveStatusOfULPDCPSDUsExtended",
                "id-ReceiveStatusOfULPDCPSDUsPDCP-SNlength18",
                "id-RecommendedCellItem",
                "id-RecommendedENBItem",
                "id-SON-Information-Report",
                "id-SignallingBasedMDTPLMNList",
                "id-Synchronisation-Information",
                "id-Time-Synchronisation-Info",
                "id-Time-UE-StayedInCell-EnhancedGranularity",
                "id-ULCOUNTValueExtended",
                "id-ULCOUNTValuePDCP-SNlength18",
                "id-eNBIndirectX2TransportLayerAddresses",
                "id-eNBX2ExtendedTransportLayerAddresses",
                "id-uE-HistoryInformationFromTheUE",
                "id-x2TNLConfigurationInfo",
                "maxEARFCN",
                "maxnoofBPLMNs",
                "maxnoofCSGs",
                "maxnoofCellID",
                "maxnoofCellIDforMDT",
                "maxnoofCellinEAI",
                "maxnoofCellinTAI",
                "maxnoofCells",
                "maxnoofCellsforRestart",
                "maxnoofCellsineNB",
                "maxnoofDCNs",
                "maxnoofE-RABs",
                "maxnoofEPLMNs",
                "maxnoofEPLMNsPlusOne",
                "maxnoofEmergencyAreaID",
                "maxnoofErrors",
                "maxnoofForbLACs",
                "maxnoofForbTACs",
                "maxnoofGroupIDs",
                "maxnoofMBSFNAreaMDT",
                "maxnoofMDTPLMNs",
                "maxnoofMMECs",
                "maxnoofPLMNsPerMME",
                "maxnoofRATs",
                "maxnoofRecommendedCells",
                "maxnoofRecommendedENBs",
                "maxnoofRestartEmergencyAreaIDs",
                "maxnoofRestartTAIs",
                "maxnoofTACs",
                "maxnoofTAIforWarning",
                "maxnoofTAforMDT",
                "maxnoofeNBX2ExtTLAs",
                "maxnoofeNBX2GTPTLAs",
                "maxnoofeNBX2TLAs",
            ],
            "S1AP-Containers": [
                "ProtocolExtensionContainer",
                "ProtocolIE-SingleContainer",
                "S1AP-PROTOCOL-EXTENSION",
                "S1AP-PROTOCOL-IES",
                "{",
                "{",
                "}",
                "}",
            ],
        },
        "object-classes": {},
        "object-sets": {
            "Additional-GUTI-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "AllocationAndRetentionPriority-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "AssistanceDataForPaging-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "AssistanceDataForRecommendedCells-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "Bearers-SubjectToStatusTransfer-ItemExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "COUNTValueExtended"},
                        "&criticality": "ignore",
                        "&id": "id-ULCOUNTValueExtended",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "COUNTValueExtended"},
                        "&criticality": "ignore",
                        "&id": "id-DLCOUNTValueExtended",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "ReceiveStatusOfULPDCPSDUsExtended"},
                        "&criticality": "ignore",
                        "&id": "id-ReceiveStatusOfULPDCPSDUsExtended",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "COUNTvaluePDCP-SNlength18"},
                        "&criticality": "ignore",
                        "&id": "id-ULCOUNTValuePDCP-SNlength18",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "COUNTvaluePDCP-SNlength18"},
                        "&criticality": "ignore",
                        "&id": "id-DLCOUNTValuePDCP-SNlength18",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {
                            "type": "ReceiveStatusOfULPDCPSDUsPDCP-SNlength18"
                        },
                        "&criticality": "ignore",
                        "&id": "id-ReceiveStatusOfULPDCPSDUsPDCP-SNlength18",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "Bearers-SubjectToStatusTransfer-ItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Bearers-SubjectToStatusTransfer-Item"},
                        "&criticality": "ignore",
                        "&id": "id-Bearers-SubjectToStatusTransfer-Item",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "CGI-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "COUNTValueExtended-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "COUNTvalue-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "COUNTvaluePDCP-SNlength18-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CSG-IdList-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CancelledCellinEAI-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CancelledCellinTAI-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "Cdma2000OneXSRVCCInfo-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellBasedMDT-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellID-Broadcast-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellID-Cancelled-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellIdentifierAndCELevelForCECapableUEs-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellType-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "CompletedCellinEAI-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CompletedCellinTAI-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CriticalityDiagnostics-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CriticalityDiagnostics-IE-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "DL-CP-SecurityInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABInformationListIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABInformationListItem"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABInformationListItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABInformationListItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABItem-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "E-RABItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABItem"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABQoSParameters-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ENB-StatusTransfer-TransparentContainer-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ENBX2ExtTLA-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "EUTRAN-CGI-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "EmergencyAreaID-Broadcast-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "EmergencyAreaID-Cancelled-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ExpectedUEActivityBehaviour-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ExpectedUEBehaviour-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ForbiddenLAs-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ForbiddenTAs-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "GBR-QosInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "GERAN-Cell-ID-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "GUMMEI-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "GlobalENB-ID-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "HandoverRestrictionList-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ImmediateMDT-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "M3Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-M3Configuration",
                        "&presence": "conditional",
                    },
                    {
                        "&Extension": {"type": "M4Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-M4Configuration",
                        "&presence": "conditional",
                    },
                    {
                        "&Extension": {"type": "M5Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-M5Configuration",
                        "&presence": "conditional",
                    },
                    {
                        "&Extension": {"type": "MDT-Location-Info"},
                        "&criticality": "ignore",
                        "&id": "id-MDT-Location-Info",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "M6Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-M6Configuration",
                        "&presence": "conditional",
                    },
                    {
                        "&Extension": {"type": "M7Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-M7Configuration",
                        "&presence": "conditional",
                    },
                    None,
                ],
            },
            "InformationForCECapableUEs-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "InformationOnRecommendedCellsAndENBsForPaging-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "LAI-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "LastVisitedEUTRANCellInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {
                            "type": "Time-UE-StayedInCell-EnhancedGranularity"
                        },
                        "&criticality": "ignore",
                        "&id": "id-Time-UE-StayedInCell-EnhancedGranularity",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-HO-Cause",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ListeningSubframePattern-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "LoggedMBSFNMDT-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "LoggedMDT-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "M1PeriodicReporting-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M1ThresholdEventA2-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M3Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M4Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M5Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M6Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "M7Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "MBSFN-ResultToLogInfo-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "MDT-Configuration-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "MDTPLMNList"},
                        "&criticality": "ignore",
                        "&id": "id-SignallingBasedMDTPLMNList",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "MDTMode-ExtensionIE": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "LoggedMBSFNMDT"},
                        "&criticality": "ignore",
                        "&id": "id-LoggedMBSFNMDT",
                        "&presence": "mandatory",
                    }
                ],
            },
            "MutingPatternInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "NB-IoT-Paging-eDRXInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "Paging-eDRXInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "PagingAttemptInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ProSeAuthorized-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "ProSeUEtoNetworkRelaying"},
                        "&criticality": "ignore",
                        "&id": "id-ProSeUEtoNetworkRelaying",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "RIMTransfer-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RLFReportInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RecommendedCellItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "RecommendedCellItem"},
                        "&criticality": "ignore",
                        "&id": "id-RecommendedCellItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "RecommendedCellsForPaging-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RecommendedCellsForPagingItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RecommendedENBItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RecommendedENBItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "RecommendedENBItem"},
                        "&criticality": "ignore",
                        "&id": "id-RecommendedENBItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "RecommendedENBsForPaging-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "RequestType-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "S-TMSI-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "SONConfigurationTransfer-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "X2TNLConfigurationInfo"},
                        "&criticality": "ignore",
                        "&id": "id-x2TNLConfigurationInfo",
                        "&presence": "conditional",
                    },
                    {
                        "&Extension": {"type": "SynchronisationInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Synchronisation-Information",
                        "&presence": "conditional",
                    },
                    None,
                ],
            },
            "SONInformation-ExtensionIE": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "SONInformationReport"},
                        "&criticality": "ignore",
                        "&id": "id-SON-Information-Report",
                        "&presence": "mandatory",
                    }
                ],
            },
            "SONInformationReply-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "TimeSynchronisationInfo"},
                        "&criticality": "ignore",
                        "&id": "id-Time-Synchronisation-Info",
                        "&presence": "optional",
                    },
                    None,
                    {
                        "&Extension": {"type": "MutingPatternInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Muting-Pattern-Information",
                        "&presence": "optional",
                    },
                ],
            },
            "SecurityContext-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ServedDCNsItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "ServedGUMMEIsItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "SourceeNB-ID-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "SourceeNB-ToTargeteNB-TransparentContainer-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "MobilityInformation"},
                        "&criticality": "ignore",
                        "&id": "id-MobilityInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "UE-HistoryInformationFromTheUE"},
                        "&criticality": "ignore",
                        "&id": "id-uE-HistoryInformationFromTheUE",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "SupportedTAs-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "RAT-Type"},
                        "&criticality": "reject",
                        "&id": "id-RAT-Type",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "SynchronisationInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TABasedMDT-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TAI-Broadcast-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TAI-Cancelled-Item-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TAI-ExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "TAIBasedMDT-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TargetRNC-ID-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TargeteNB-ID-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TargeteNB-ToSourceeNB-TransparentContainer-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "TimeSynchronisationInfo-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "MutingAvailabilityIndication"},
                        "&criticality": "ignore",
                        "&id": "id-Muting-Availability-Indication",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "TraceActivation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "MDT-Configuration"},
                        "&criticality": "ignore",
                        "&id": "id-MDTConfiguration",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "Tunnel-Information-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UE-S1AP-ID-pair-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UE-Sidelink-Aggregate-MaximumBitrates-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UE-associatedLogicalS1-ConnectionItemExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UEAggregate-MaximumBitrates-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UESecurityCapabilities-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UL-CP-SecurityInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "UserLocationInformation-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "V2XServicesAuthorized-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "X2TNLConfigurationInfo-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "ENBX2ExtTLAs"},
                        "&criticality": "ignore",
                        "&id": "id-eNBX2ExtendedTransportLayerAddresses",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "ENBIndirectX2TransportLayerAddresses"},
                        "&criticality": "ignore",
                        "&id": "id-eNBIndirectX2TransportLayerAddresses",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
        },
        "tags": "AUTOMATIC",
        "types": {
            "Additional-GUTI": {
                "members": [
                    {"name": "gUMMEI", "type": "GUMMEI"},
                    {"name": "m-TMSI", "type": "M-TMSI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["Additional-GUTI-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "AdditionalCSFallbackIndicator": {
                "type": "ENUMERATED",
                "values": [("no-restriction", 0), ("restriction", 1), None],
            },
            "AllocationAndRetentionPriority": {
                "members": [
                    {"name": "priorityLevel", "type": "PriorityLevel"},
                    {"name": "pre-emptionCapability", "type": "Pre-emptionCapability"},
                    {
                        "name": "pre-emptionVulnerability",
                        "type": "Pre-emptionVulnerability",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["AllocationAndRetentionPriority-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "AreaScopeOfMDT": {
                "members": [
                    {"name": "cellBased", "type": "CellBasedMDT"},
                    {"name": "tABased", "type": "TABasedMDT"},
                    {"name": "pLMNWide", "type": "NULL"},
                    None,
                    {"name": "tAIBased", "type": "TAIBasedMDT"},
                ],
                "type": "CHOICE",
            },
            "AssistanceDataForCECapableUEs": {
                "members": [
                    {
                        "name": "cellIdentifierAndCELevelForCECapableUEs",
                        "type": "CellIdentifierAndCELevelForCECapableUEs",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["InformationForCECapableUEs-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "AssistanceDataForPaging": {
                "members": [
                    {
                        "name": "assistanceDataForRecommendedCells",
                        "optional": True,
                        "type": "AssistanceDataForRecommendedCells",
                    },
                    {
                        "name": "assistanceDataForCECapableUEs",
                        "optional": True,
                        "type": "AssistanceDataForCECapableUEs",
                    },
                    {
                        "name": "pagingAttemptInformation",
                        "optional": True,
                        "type": "PagingAttemptInformation",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["AssistanceDataForPaging-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "AssistanceDataForRecommendedCells": {
                "members": [
                    {
                        "name": "recommendedCellsForPaging",
                        "type": "RecommendedCellsForPaging",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["AssistanceDataForRecommendedCells-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "BPLMNs": {
                "element": {"type": "PLMNidentity"},
                "size": [(1, "maxnoofBPLMNs")],
                "type": "SEQUENCE OF",
            },
            "BearerType": {"type": "ENUMERATED", "values": [("non-IP", 0), None]},
            "Bearers-SubjectToStatusTransfer-Item": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "uL-COUNTvalue", "type": "COUNTvalue"},
                    {"name": "dL-COUNTvalue", "type": "COUNTvalue"},
                    {
                        "name": "receiveStatusofULPDCPSDUs",
                        "optional": True,
                        "type": "ReceiveStatusofULPDCPSDUs",
                    },
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "Bearers-SubjectToStatusTransfer-ItemExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Bearers-SubjectToStatusTransferList": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["Bearers-SubjectToStatusTransfer-ItemIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "BitRate": {"restricted-to": [(0, 10000000000)], "type": "INTEGER"},
            "BroadcastCancelledAreaList": {
                "members": [
                    {"name": "cellID-Cancelled", "type": "CellID-Cancelled"},
                    {"name": "tAI-Cancelled", "type": "TAI-Cancelled"},
                    {
                        "name": "emergencyAreaID-Cancelled",
                        "type": "EmergencyAreaID-Cancelled",
                    },
                    None,
                ],
                "type": "CHOICE",
            },
            "BroadcastCompletedAreaList": {
                "members": [
                    {"name": "cellID-Broadcast", "type": "CellID-Broadcast"},
                    {"name": "tAI-Broadcast", "type": "TAI-Broadcast"},
                    {
                        "name": "emergencyAreaID-Broadcast",
                        "type": "EmergencyAreaID-Broadcast",
                    },
                    None,
                ],
                "type": "CHOICE",
            },
            "CE-mode-B-SupportIndicator": {
                "type": "ENUMERATED",
                "values": [("supported", 0), None],
            },
            "CELevel": {"type": "OCTET STRING"},
            "CGI": {
                "members": [
                    {"name": "pLMNidentity", "type": "PLMNidentity"},
                    {"name": "lAC", "type": "LAC"},
                    {"name": "cI", "type": "CI"},
                    {"name": "rAC", "optional": True, "type": "RAC"},
                    {
                        "actual-parameters": [{"object-set": ["CGI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CI": {"size": [2], "type": "OCTET STRING"},
            "CNDomain": {"type": "ENUMERATED", "values": [("ps", 0), ("cs", 1)]},
            "COUNTValueExtended": {
                "members": [
                    {"name": "pDCP-SNExtended", "type": "PDCP-SNExtended"},
                    {"name": "hFNModified", "type": "HFNModified"},
                    {
                        "actual-parameters": [
                            {"object-set": ["COUNTValueExtended-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "COUNTvalue": {
                "members": [
                    {"name": "pDCP-SN", "type": "PDCP-SN"},
                    {"name": "hFN", "type": "HFN"},
                    {
                        "actual-parameters": [{"object-set": ["COUNTvalue-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "COUNTvaluePDCP-SNlength18": {
                "members": [
                    {"name": "pDCP-SNlength18", "type": "PDCP-SNlength18"},
                    {"name": "hFNforPDCP-SNlength18", "type": "HFNforPDCP-SNlength18"},
                    {
                        "actual-parameters": [
                            {"object-set": ["COUNTvaluePDCP-SNlength18-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CSFallbackIndicator": {
                "type": "ENUMERATED",
                "values": [
                    ("cs-fallback-required", 0),
                    None,
                    ("cs-fallback-high-priority", 1),
                ],
            },
            "CSG-Id": {"size": [27], "type": "BIT STRING"},
            "CSG-IdList": {
                "element": {"type": "CSG-IdList-Item"},
                "size": [(1, "maxnoofCSGs")],
                "type": "SEQUENCE OF",
            },
            "CSG-IdList-Item": {
                "members": [
                    {"name": "cSG-Id", "type": "CSG-Id"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CSG-IdList-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CSGMembershipStatus": {
                "type": "ENUMERATED",
                "values": [("member", 0), ("not-member", 1)],
            },
            "CancelledCellinEAI": {
                "element": {"type": "CancelledCellinEAI-Item"},
                "size": [(1, "maxnoofCellinEAI")],
                "type": "SEQUENCE OF",
            },
            "CancelledCellinEAI-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {"name": "numberOfBroadcasts", "type": "NumberOfBroadcasts"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CancelledCellinEAI-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CancelledCellinTAI": {
                "element": {"type": "CancelledCellinTAI-Item"},
                "size": [(1, "maxnoofCellinTAI")],
                "type": "SEQUENCE OF",
            },
            "CancelledCellinTAI-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {"name": "numberOfBroadcasts", "type": "NumberOfBroadcasts"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CancelledCellinTAI-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Cause": {
                "members": [
                    {"name": "radioNetwork", "type": "CauseRadioNetwork"},
                    {"name": "transport", "type": "CauseTransport"},
                    {"name": "nas", "type": "CauseNas"},
                    {"name": "protocol", "type": "CauseProtocol"},
                    {"name": "misc", "type": "CauseMisc"},
                    None,
                ],
                "type": "CHOICE",
            },
            "CauseMisc": {
                "type": "ENUMERATED",
                "values": [
                    ("control-processing-overload", 0),
                    ("not-enough-user-plane-processing-resources", 1),
                    ("hardware-failure", 2),
                    ("om-intervention", 3),
                    ("unspecified", 4),
                    ("unknown-PLMN", 5),
                    None,
                ],
            },
            "CauseNas": {
                "type": "ENUMERATED",
                "values": [
                    ("normal-release", 0),
                    ("authentication-failure", 1),
                    ("detach", 2),
                    ("unspecified", 3),
                    None,
                    ("csg-subscription-expiry", 4),
                ],
            },
            "CauseProtocol": {
                "type": "ENUMERATED",
                "values": [
                    ("transfer-syntax-error", 0),
                    ("abstract-syntax-error-reject", 1),
                    ("abstract-syntax-error-ignore-and-notify", 2),
                    ("message-not-compatible-with-receiver-state", 3),
                    ("semantic-error", 4),
                    ("abstract-syntax-error-falsely-constructed-message", 5),
                    ("unspecified", 6),
                    None,
                ],
            },
            "CauseRadioNetwork": {
                "type": "ENUMERATED",
                "values": [
                    ("unspecified", 0),
                    ("tx2relocoverall-expiry", 1),
                    ("successful-handover", 2),
                    ("release-due-to-eutran-generated-reason", 3),
                    ("handover-cancelled", 4),
                    ("partial-handover", 5),
                    ("ho-failure-in-target-EPC-eNB-or-target-system", 6),
                    ("ho-target-not-allowed", 7),
                    ("tS1relocoverall-expiry", 8),
                    ("tS1relocprep-expiry", 9),
                    ("cell-not-available", 10),
                    ("unknown-targetID", 11),
                    ("no-radio-resources-available-in-target-cell", 12),
                    ("unknown-mme-ue-s1ap-id", 13),
                    ("unknown-enb-ue-s1ap-id", 14),
                    ("unknown-pair-ue-s1ap-id", 15),
                    ("handover-desirable-for-radio-reason", 16),
                    ("time-critical-handover", 17),
                    ("resource-optimisation-handover", 18),
                    ("reduce-load-in-serving-cell", 19),
                    ("user-inactivity", 20),
                    ("radio-connection-with-ue-lost", 21),
                    ("load-balancing-tau-required", 22),
                    ("cs-fallback-triggered", 23),
                    ("ue-not-available-for-ps-service", 24),
                    ("radio-resources-not-available", 25),
                    ("failure-in-radio-interface-procedure", 26),
                    ("invalid-qos-combination", 27),
                    ("interrat-redirection", 28),
                    ("interaction-with-other-procedure", 29),
                    ("unknown-E-RAB-ID", 30),
                    ("multiple-E-RAB-ID-instances", 31),
                    (
                        "encryption-and-or-integrity-protection-algorithms-not-supported",
                        32,
                    ),
                    ("s1-intra-system-handover-triggered", 33),
                    ("s1-inter-system-handover-triggered", 34),
                    ("x2-handover-triggered", 35),
                    None,
                    ("redirection-towards-1xRTT", 36),
                    ("not-supported-QCI-value", 37),
                    ("invalid-CSG-Id", 38),
                    ("release-due-to-pre-emption", 39),
                ],
            },
            "CauseTransport": {
                "type": "ENUMERATED",
                "values": [
                    ("transport-resource-unavailable", 0),
                    ("unspecified", 1),
                    None,
                ],
            },
            "Cdma2000HORequiredIndication": {
                "type": "ENUMERATED",
                "values": [("true", 0), None],
            },
            "Cdma2000HOStatus": {
                "type": "ENUMERATED",
                "values": [("hOSuccess", 0), ("hOFailure", 1), None],
            },
            "Cdma2000OneXMEID": {"type": "OCTET STRING"},
            "Cdma2000OneXMSI": {"type": "OCTET STRING"},
            "Cdma2000OneXPilot": {"type": "OCTET STRING"},
            "Cdma2000OneXRAND": {"type": "OCTET STRING"},
            "Cdma2000OneXSRVCCInfo": {
                "members": [
                    {"name": "cdma2000OneXMEID", "type": "Cdma2000OneXMEID"},
                    {"name": "cdma2000OneXMSI", "type": "Cdma2000OneXMSI"},
                    {"name": "cdma2000OneXPilot", "type": "Cdma2000OneXPilot"},
                    {
                        "actual-parameters": [
                            {"object-set": ["Cdma2000OneXSRVCCInfo-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Cdma2000PDU": {"type": "OCTET STRING"},
            "Cdma2000RATType": {
                "type": "ENUMERATED",
                "values": [("hRPD", 0), ("onexRTT", 1), None],
            },
            "Cdma2000SectorID": {"type": "OCTET STRING"},
            "Cell-Size": {
                "type": "ENUMERATED",
                "values": [
                    ("verysmall", 0),
                    ("small", 1),
                    ("medium", 2),
                    ("large", 3),
                    None,
                ],
            },
            "CellAccessMode": {"type": "ENUMERATED", "values": [("hybrid", 0), None]},
            "CellBasedMDT": {
                "members": [
                    {"name": "cellIdListforMDT", "type": "CellIdListforMDT"},
                    {
                        "actual-parameters": [{"object-set": ["CellBasedMDT-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CellID-Broadcast": {
                "element": {"type": "CellID-Broadcast-Item"},
                "size": [(1, "maxnoofCellID")],
                "type": "SEQUENCE OF",
            },
            "CellID-Broadcast-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CellID-Broadcast-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CellID-Cancelled": {
                "element": {"type": "CellID-Cancelled-Item"},
                "size": [(1, "maxnoofCellID")],
                "type": "SEQUENCE OF",
            },
            "CellID-Cancelled-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {"name": "numberOfBroadcasts", "type": "NumberOfBroadcasts"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CellID-Cancelled-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CellIdListforMDT": {
                "element": {"type": "EUTRAN-CGI"},
                "size": [(1, "maxnoofCellIDforMDT")],
                "type": "SEQUENCE OF",
            },
            "CellIdentifierAndCELevelForCECapableUEs": {
                "members": [
                    {"name": "global-Cell-ID", "type": "EUTRAN-CGI"},
                    {"name": "cELevel", "type": "CELevel"},
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "CellIdentifierAndCELevelForCECapableUEs-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CellIdentity": {"size": [28], "type": "BIT STRING"},
            "CellType": {
                "members": [
                    {"name": "cell-Size", "type": "Cell-Size"},
                    {
                        "actual-parameters": [{"object-set": ["CellType-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CompletedCellinEAI": {
                "element": {"type": "CompletedCellinEAI-Item"},
                "size": [(1, "maxnoofCellinEAI")],
                "type": "SEQUENCE OF",
            },
            "CompletedCellinEAI-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CompletedCellinEAI-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CompletedCellinTAI": {
                "element": {"type": "CompletedCellinTAI-Item"},
                "size": [(1, "maxnoofCellinTAI")],
                "type": "SEQUENCE OF",
            },
            "CompletedCellinTAI-Item": {
                "members": [
                    {"name": "eCGI", "type": "EUTRAN-CGI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CompletedCellinTAI-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ConcurrentWarningMessageIndicator": {
                "type": "ENUMERATED",
                "values": [("true", 0)],
            },
            "Correlation-ID": {"size": [4], "type": "OCTET STRING"},
            "Coverage-Level": {
                "type": "ENUMERATED",
                "values": [("extendedcoverage", 0), None],
            },
            "CriticalityDiagnostics": {
                "members": [
                    {
                        "name": "procedureCode",
                        "optional": True,
                        "type": "ProcedureCode",
                    },
                    {
                        "name": "triggeringMessage",
                        "optional": True,
                        "type": "TriggeringMessage",
                    },
                    {
                        "name": "procedureCriticality",
                        "optional": True,
                        "type": "Criticality",
                    },
                    {
                        "name": "iEsCriticalityDiagnostics",
                        "optional": True,
                        "type": "CriticalityDiagnostics-IE-List",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["CriticalityDiagnostics-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CriticalityDiagnostics-IE-Item": {
                "members": [
                    {"name": "iECriticality", "type": "Criticality"},
                    {"name": "iE-ID", "type": "ProtocolIE-ID"},
                    {"name": "typeOfError", "type": "TypeOfError"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CriticalityDiagnostics-IE-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CriticalityDiagnostics-IE-List": {
                "element": {"type": "CriticalityDiagnostics-IE-Item"},
                "size": [(1, "maxnoofErrors")],
                "type": "SEQUENCE OF",
            },
            "DCN-ID": {"restricted-to": [(0, 65535)], "type": "INTEGER"},
            "DL-CP-SecurityInformation": {
                "members": [
                    {"name": "dl-NAS-MAC", "type": "DL-NAS-MAC"},
                    {
                        "actual-parameters": [
                            {"object-set": ["DL-CP-SecurityInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DL-Forwarding": {
                "type": "ENUMERATED",
                "values": [("dL-Forwarding-proposed", 0), None],
            },
            "DL-NAS-MAC": {"size": [16], "type": "BIT STRING"},
            "DLNASPDUDeliveryAckRequest": {
                "type": "ENUMERATED",
                "values": [("requested", 0), None],
            },
            "Data-Forwarding-Not-Possible": {
                "type": "ENUMERATED",
                "values": [("data-Forwarding-not-Possible", 0), None],
            },
            "DataCodingScheme": {"size": [8], "type": "BIT STRING"},
            "Direct-Forwarding-Path-Availability": {
                "type": "ENUMERATED",
                "values": [("directPathAvailable", 0), None],
            },
            "E-RAB-ID": {"restricted-to": [(0, 15), None], "type": "INTEGER"},
            "E-RABInformationList": {
                "element": {
                    "actual-parameters": [{"object-set": ["E-RABInformationListIEs"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABInformationListItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "name": "dL-Forwarding",
                        "optional": True,
                        "type": "DL-Forwarding",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABInformationListItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "cause", "type": "Cause"},
                    {
                        "actual-parameters": [{"object-set": ["E-RABItem-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABLevelQoSParameters": {
                "members": [
                    {"name": "qCI", "type": "QCI"},
                    {
                        "name": "allocationRetentionPriority",
                        "type": "AllocationAndRetentionPriority",
                    },
                    {
                        "name": "gbrQosInformation",
                        "optional": True,
                        "type": "GBR-QosInformation",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABQoSParameters-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABList": {
                "element": {
                    "actual-parameters": [{"object-set": ["E-RABItemIEs"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-UTRAN-Trace-ID": {"size": [8], "type": "OCTET STRING"},
            "EARFCN": {"restricted-to": [(0, "maxEARFCN"), None], "type": "INTEGER"},
            "ECGI-List": {
                "element": {"type": "EUTRAN-CGI"},
                "size": [(1, "maxnoofCellsineNB")],
                "type": "SEQUENCE OF",
            },
            "ECGIList": {
                "element": {"type": "EUTRAN-CGI"},
                "size": [(1, "maxnoofCellID")],
                "type": "SEQUENCE OF",
            },
            "ECGIListForRestart": {
                "element": {"type": "EUTRAN-CGI"},
                "size": [(1, "maxnoofCellsforRestart")],
                "type": "SEQUENCE OF",
            },
            "ENB-ID": {
                "members": [
                    {"name": "macroENB-ID", "size": [20], "type": "BIT STRING"},
                    {"name": "homeENB-ID", "size": [28], "type": "BIT STRING"},
                    None,
                    {"name": "short-macroENB-ID", "size": [18], "type": "BIT STRING"},
                    {"name": "long-macroENB-ID", "size": [21], "type": "BIT STRING"},
                ],
                "type": "CHOICE",
            },
            "ENB-StatusTransfer-TransparentContainer": {
                "members": [
                    {
                        "name": "bearers-SubjectToStatusTransferList",
                        "type": "Bearers-SubjectToStatusTransferList",
                    },
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "ENB-StatusTransfer-TransparentContainer-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENB-UE-S1AP-ID": {"restricted-to": [(0, 16777215)], "type": "INTEGER"},
            "ENBIndirectX2TransportLayerAddresses": {
                "element": {"type": "TransportLayerAddress"},
                "size": [(1, "maxnoofeNBX2TLAs")],
                "type": "SEQUENCE OF",
            },
            "ENBX2ExtTLA": {
                "members": [
                    {
                        "name": "iPsecTLA",
                        "optional": True,
                        "type": "TransportLayerAddress",
                    },
                    {"name": "gTPTLAa", "optional": True, "type": "ENBX2GTPTLAs"},
                    {
                        "actual-parameters": [{"object-set": ["ENBX2ExtTLA-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBX2ExtTLAs": {
                "element": {"type": "ENBX2ExtTLA"},
                "size": [(1, "maxnoofeNBX2ExtTLAs")],
                "type": "SEQUENCE OF",
            },
            "ENBX2GTPTLAs": {
                "element": {"type": "TransportLayerAddress"},
                "size": [(1, "maxnoofeNBX2GTPTLAs")],
                "type": "SEQUENCE OF",
            },
            "ENBX2TLAs": {
                "element": {"type": "TransportLayerAddress"},
                "size": [(1, "maxnoofeNBX2TLAs")],
                "type": "SEQUENCE OF",
            },
            "ENBname": {"size": [(1, 150), None], "type": "PrintableString"},
            "EPLMNs": {
                "element": {"type": "PLMNidentity"},
                "size": [(1, "maxnoofEPLMNs")],
                "type": "SEQUENCE OF",
            },
            "EUTRAN-CGI": {
                "members": [
                    {"name": "pLMNidentity", "type": "PLMNidentity"},
                    {"name": "cell-ID", "type": "CellIdentity"},
                    {
                        "actual-parameters": [{"object-set": ["EUTRAN-CGI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "EUTRANRoundTripDelayEstimationInfo": {
                "restricted-to": [(0, 2047)],
                "type": "INTEGER",
            },
            "EmergencyAreaID": {"size": [3], "type": "OCTET STRING"},
            "EmergencyAreaID-Broadcast": {
                "element": {"type": "EmergencyAreaID-Broadcast-Item"},
                "size": [(1, "maxnoofEmergencyAreaID")],
                "type": "SEQUENCE OF",
            },
            "EmergencyAreaID-Broadcast-Item": {
                "members": [
                    {"name": "emergencyAreaID", "type": "EmergencyAreaID"},
                    {"name": "completedCellinEAI", "type": "CompletedCellinEAI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["EmergencyAreaID-Broadcast-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "EmergencyAreaID-Cancelled": {
                "element": {"type": "EmergencyAreaID-Cancelled-Item"},
                "size": [(1, "maxnoofEmergencyAreaID")],
                "type": "SEQUENCE OF",
            },
            "EmergencyAreaID-Cancelled-Item": {
                "members": [
                    {"name": "emergencyAreaID", "type": "EmergencyAreaID"},
                    {"name": "cancelledCellinEAI", "type": "CancelledCellinEAI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["EmergencyAreaID-Cancelled-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "EmergencyAreaIDList": {
                "element": {"type": "EmergencyAreaID"},
                "size": [(1, "maxnoofEmergencyAreaID")],
                "type": "SEQUENCE OF",
            },
            "EmergencyAreaIDListForRestart": {
                "element": {"type": "EmergencyAreaID"},
                "size": [(1, "maxnoofRestartEmergencyAreaIDs")],
                "type": "SEQUENCE OF",
            },
            "EncryptionAlgorithms": {"size": [16, None], "type": "BIT STRING"},
            "EnhancedCoverageRestricted": {
                "type": "ENUMERATED",
                "values": [("restricted", 0), None],
            },
            "EventType": {
                "type": "ENUMERATED",
                "values": [
                    ("direct", 0),
                    ("change-of-serve-cell", 1),
                    ("stop-change-of-serve-cell", 2),
                    None,
                ],
            },
            "ExpectedActivityPeriod": {
                "restricted-to": [
                    (1, 30),
                    40,
                    50,
                    60,
                    80,
                    100,
                    120,
                    150,
                    180,
                    181,
                    None,
                ],
                "type": "INTEGER",
            },
            "ExpectedHOInterval": {
                "type": "ENUMERATED",
                "values": [
                    ("sec15", 0),
                    ("sec30", 1),
                    ("sec60", 2),
                    ("sec90", 3),
                    ("sec120", 4),
                    ("sec180", 5),
                    ("long-time", 6),
                    None,
                ],
            },
            "ExpectedIdlePeriod": {
                "restricted-to": [
                    (1, 30),
                    40,
                    50,
                    60,
                    80,
                    100,
                    120,
                    150,
                    180,
                    181,
                    None,
                ],
                "type": "INTEGER",
            },
            "ExpectedUEActivityBehaviour": {
                "members": [
                    {
                        "name": "expectedActivityPeriod",
                        "optional": True,
                        "type": "ExpectedActivityPeriod",
                    },
                    {
                        "name": "expectedIdlePeriod",
                        "optional": True,
                        "type": "ExpectedIdlePeriod",
                    },
                    {
                        "name": "sourceofUEActivityBehaviourInformation",
                        "optional": True,
                        "type": "SourceOfUEActivityBehaviourInformation",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["ExpectedUEActivityBehaviour-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ExpectedUEBehaviour": {
                "members": [
                    {
                        "name": "expectedActivity",
                        "optional": True,
                        "type": "ExpectedUEActivityBehaviour",
                    },
                    {
                        "name": "expectedHOInterval",
                        "optional": True,
                        "type": "ExpectedHOInterval",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["ExpectedUEBehaviour-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Extended-UEIdentityIndexValue": {"size": [14], "type": "BIT STRING"},
            "ExtendedRNC-ID": {"restricted-to": [(4096, 65535)], "type": "INTEGER"},
            "ExtendedRepetitionPeriod": {
                "restricted-to": [(4096, 131071)],
                "type": "INTEGER",
            },
            "ForbiddenInterRATs": {
                "type": "ENUMERATED",
                "values": [
                    ("all", 0),
                    ("geran", 1),
                    ("utran", 2),
                    ("cdma2000", 3),
                    None,
                    ("geranandutran", 4),
                    ("cdma2000andutran", 5),
                ],
            },
            "ForbiddenLACs": {
                "element": {"type": "LAC"},
                "size": [(1, "maxnoofForbLACs")],
                "type": "SEQUENCE OF",
            },
            "ForbiddenLAs": {
                "element": {"type": "ForbiddenLAs-Item"},
                "size": [(1, "maxnoofEPLMNsPlusOne")],
                "type": "SEQUENCE OF",
            },
            "ForbiddenLAs-Item": {
                "members": [
                    {"name": "pLMN-Identity", "type": "PLMNidentity"},
                    {"name": "forbiddenLACs", "type": "ForbiddenLACs"},
                    {
                        "actual-parameters": [
                            {"object-set": ["ForbiddenLAs-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ForbiddenTACs": {
                "element": {"type": "TAC"},
                "size": [(1, "maxnoofForbTACs")],
                "type": "SEQUENCE OF",
            },
            "ForbiddenTAs": {
                "element": {"type": "ForbiddenTAs-Item"},
                "size": [(1, "maxnoofEPLMNsPlusOne")],
                "type": "SEQUENCE OF",
            },
            "ForbiddenTAs-Item": {
                "members": [
                    {"name": "pLMN-Identity", "type": "PLMNidentity"},
                    {"name": "forbiddenTACs", "type": "ForbiddenTACs"},
                    {
                        "actual-parameters": [
                            {"object-set": ["ForbiddenTAs-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "GBR-QosInformation": {
                "members": [
                    {"name": "e-RAB-MaximumBitrateDL", "type": "BitRate"},
                    {"name": "e-RAB-MaximumBitrateUL", "type": "BitRate"},
                    {"name": "e-RAB-GuaranteedBitrateDL", "type": "BitRate"},
                    {"name": "e-RAB-GuaranteedBitrateUL", "type": "BitRate"},
                    {
                        "actual-parameters": [
                            {"object-set": ["GBR-QosInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "GERAN-Cell-ID": {
                "members": [
                    {"name": "lAI", "type": "LAI"},
                    {"name": "rAC", "type": "RAC"},
                    {"name": "cI", "type": "CI"},
                    {
                        "actual-parameters": [{"object-set": ["GERAN-Cell-ID-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "GTP-TEID": {"size": [4], "type": "OCTET STRING"},
            "GUMMEI": {
                "members": [
                    {"name": "pLMN-Identity", "type": "PLMNidentity"},
                    {"name": "mME-Group-ID", "type": "MME-Group-ID"},
                    {"name": "mME-Code", "type": "MME-Code"},
                    {
                        "actual-parameters": [{"object-set": ["GUMMEI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "GUMMEIList": {
                "element": {"type": "GUMMEI"},
                "size": [(1, "maxnoofMMECs")],
                "type": "SEQUENCE OF",
            },
            "GUMMEIType": {
                "type": "ENUMERATED",
                "values": [("native", 0), ("mapped", 1), None],
            },
            "GWContextReleaseIndication": {
                "type": "ENUMERATED",
                "values": [("true", 0), None],
            },
            "Global-ENB-ID": {
                "members": [
                    {"name": "pLMNidentity", "type": "PLMNidentity"},
                    {"name": "eNB-ID", "type": "ENB-ID"},
                    {
                        "actual-parameters": [{"object-set": ["GlobalENB-ID-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HFN": {"restricted-to": [(0, 1048575)], "type": "INTEGER"},
            "HFNModified": {"restricted-to": [(0, 131071)], "type": "INTEGER"},
            "HFNforPDCP-SNlength18": {"restricted-to": [(0, 16383)], "type": "INTEGER"},
            "HandoverRestrictionList": {
                "members": [
                    {"name": "servingPLMN", "type": "PLMNidentity"},
                    {"name": "equivalentPLMNs", "optional": True, "type": "EPLMNs"},
                    {"name": "forbiddenTAs", "optional": True, "type": "ForbiddenTAs"},
                    {"name": "forbiddenLAs", "optional": True, "type": "ForbiddenLAs"},
                    {
                        "name": "forbiddenInterRATs",
                        "optional": True,
                        "type": "ForbiddenInterRATs",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["HandoverRestrictionList-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverType": {
                "type": "ENUMERATED",
                "values": [
                    ("intralte", 0),
                    ("ltetoutran", 1),
                    ("ltetogeran", 2),
                    ("utrantolte", 3),
                    ("gerantolte", 4),
                    None,
                ],
            },
            "IMSI": {"size": [(3, 8)], "type": "OCTET STRING"},
            "ImmediateMDT": {
                "members": [
                    {
                        "name": "measurementsToActivate",
                        "type": "MeasurementsToActivate",
                    },
                    {"name": "m1reportingTrigger", "type": "M1ReportingTrigger"},
                    {
                        "name": "m1thresholdeventA2",
                        "optional": True,
                        "type": "M1ThresholdEventA2",
                    },
                    {
                        "name": "m1periodicReporting",
                        "optional": True,
                        "type": "M1PeriodicReporting",
                    },
                    {
                        "actual-parameters": [{"object-set": ["ImmediateMDT-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "InformationOnRecommendedCellsAndENBsForPaging": {
                "members": [
                    {
                        "name": "recommendedCellsForPaging",
                        "type": "RecommendedCellsForPaging",
                    },
                    {
                        "name": "recommendENBsForPaging",
                        "type": "RecommendedENBsForPaging",
                    },
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "InformationOnRecommendedCellsAndENBsForPaging-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "IntegrityProtectionAlgorithms": {"size": [16, None], "type": "BIT STRING"},
            "IntendedNumberOfPagingAttempts": {
                "restricted-to": [(1, 16), None],
                "type": "INTEGER",
            },
            "InterfacesToTrace": {"size": [8], "type": "BIT STRING"},
            "KillAllWarningMessages": {"type": "ENUMERATED", "values": [("true", 0)]},
            "L3-Information": {"type": "OCTET STRING"},
            "LAC": {"size": [2], "type": "OCTET STRING"},
            "LAI": {
                "members": [
                    {"name": "pLMNidentity", "type": "PLMNidentity"},
                    {"name": "lAC", "type": "LAC"},
                    {
                        "actual-parameters": [{"object-set": ["LAI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LHN-ID": {"size": [(32, 256)], "type": "OCTET STRING"},
            "LPPa-PDU": {"type": "OCTET STRING"},
            "LastVisitedCell-Item": {
                "members": [
                    {
                        "name": "e-UTRAN-Cell",
                        "type": "LastVisitedEUTRANCellInformation",
                    },
                    {"name": "uTRAN-Cell", "type": "LastVisitedUTRANCellInformation"},
                    {"name": "gERAN-Cell", "type": "LastVisitedGERANCellInformation"},
                    None,
                ],
                "type": "CHOICE",
            },
            "LastVisitedEUTRANCellInformation": {
                "members": [
                    {"name": "global-Cell-ID", "type": "EUTRAN-CGI"},
                    {"name": "cellType", "type": "CellType"},
                    {"name": "time-UE-StayedInCell", "type": "Time-UE-StayedInCell"},
                    {
                        "actual-parameters": [
                            {"object-set": ["LastVisitedEUTRANCellInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LastVisitedGERANCellInformation": {
                "members": [{"name": "undefined", "type": "NULL"}, None],
                "type": "CHOICE",
            },
            "LastVisitedUTRANCellInformation": {"type": "OCTET STRING"},
            "Links-to-log": {
                "type": "ENUMERATED",
                "values": [
                    ("uplink", 0),
                    ("downlink", 1),
                    ("both-uplink-and-downlink", 2),
                    None,
                ],
            },
            "ListeningSubframePattern": {
                "members": [
                    {
                        "name": "pattern-period",
                        "type": "ENUMERATED",
                        "values": [
                            ("ms1280", 0),
                            ("ms2560", 1),
                            ("ms5120", 2),
                            ("ms10240", 3),
                            None,
                        ],
                    },
                    {
                        "name": "pattern-offset",
                        "restricted-to": [(0, 10239), None],
                        "type": "INTEGER",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["ListeningSubframePattern-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LoggedMBSFNMDT": {
                "members": [
                    {"name": "loggingInterval", "type": "LoggingInterval"},
                    {"name": "loggingDuration", "type": "LoggingDuration"},
                    {
                        "name": "mBSFN-ResultToLog",
                        "optional": True,
                        "type": "MBSFN-ResultToLog",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["LoggedMBSFNMDT-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LoggedMDT": {
                "members": [
                    {"name": "loggingInterval", "type": "LoggingInterval"},
                    {"name": "loggingDuration", "type": "LoggingDuration"},
                    {
                        "actual-parameters": [{"object-set": ["LoggedMDT-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LoggingDuration": {
                "type": "ENUMERATED",
                "values": [
                    ("m10", 0),
                    ("m20", 1),
                    ("m40", 2),
                    ("m60", 3),
                    ("m90", 4),
                    ("m120", 5),
                ],
            },
            "LoggingInterval": {
                "type": "ENUMERATED",
                "values": [
                    ("ms128", 0),
                    ("ms256", 1),
                    ("ms512", 2),
                    ("ms1024", 3),
                    ("ms2048", 4),
                    ("ms3072", 5),
                    ("ms4096", 6),
                    ("ms6144", 7),
                ],
            },
            "M-TMSI": {"size": [4], "type": "OCTET STRING"},
            "M1PeriodicReporting": {
                "members": [
                    {"name": "reportInterval", "type": "ReportIntervalMDT"},
                    {"name": "reportAmount", "type": "ReportAmountMDT"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M1PeriodicReporting-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M1ReportingTrigger": {
                "type": "ENUMERATED",
                "values": [
                    ("periodic", 0),
                    ("a2eventtriggered", 1),
                    None,
                    ("a2eventtriggered-periodic", 2),
                ],
            },
            "M1ThresholdEventA2": {
                "members": [
                    {"name": "measurementThreshold", "type": "MeasurementThresholdA2"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M1ThresholdEventA2-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M3Configuration": {
                "members": [
                    {"name": "m3period", "type": "M3period"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M3Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M3period": {
                "type": "ENUMERATED",
                "values": [("ms100", 0), ("ms1000", 1), ("ms10000", 2), None],
            },
            "M4Configuration": {
                "members": [
                    {"name": "m4period", "type": "M4period"},
                    {"name": "m4-links-to-log", "type": "Links-to-log"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M4Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M4period": {
                "type": "ENUMERATED",
                "values": [
                    ("ms1024", 0),
                    ("ms2048", 1),
                    ("ms5120", 2),
                    ("ms10240", 3),
                    ("min1", 4),
                    None,
                ],
            },
            "M5Configuration": {
                "members": [
                    {"name": "m5period", "type": "M5period"},
                    {"name": "m5-links-to-log", "type": "Links-to-log"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M5Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M5period": {
                "type": "ENUMERATED",
                "values": [
                    ("ms1024", 0),
                    ("ms2048", 1),
                    ("ms5120", 2),
                    ("ms10240", 3),
                    ("min1", 4),
                    None,
                ],
            },
            "M6Configuration": {
                "members": [
                    {"name": "m6report-Interval", "type": "M6report-Interval"},
                    {
                        "name": "m6delay-threshold",
                        "optional": True,
                        "type": "M6delay-threshold",
                    },
                    {"name": "m6-links-to-log", "type": "Links-to-log"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M6Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M6delay-threshold": {
                "type": "ENUMERATED",
                "values": [
                    ("ms30", 0),
                    ("ms40", 1),
                    ("ms50", 2),
                    ("ms60", 3),
                    ("ms70", 4),
                    ("ms80", 5),
                    ("ms90", 6),
                    ("ms100", 7),
                    ("ms150", 8),
                    ("ms300", 9),
                    ("ms500", 10),
                    ("ms750", 11),
                    None,
                ],
            },
            "M6report-Interval": {
                "type": "ENUMERATED",
                "values": [
                    ("ms1024", 0),
                    ("ms2048", 1),
                    ("ms5120", 2),
                    ("ms10240", 3),
                    None,
                ],
            },
            "M7Configuration": {
                "members": [
                    {"name": "m7period", "type": "M7period"},
                    {"name": "m7-links-to-log", "type": "Links-to-log"},
                    {
                        "actual-parameters": [
                            {"object-set": ["M7Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "M7period": {"restricted-to": [(1, 60), None], "type": "INTEGER"},
            "MBSFN-ResultToLog": {
                "element": {"type": "MBSFN-ResultToLogInfo"},
                "size": [(1, "maxnoofMBSFNAreaMDT")],
                "type": "SEQUENCE OF",
            },
            "MBSFN-ResultToLogInfo": {
                "members": [
                    {
                        "name": "mBSFN-AreaId",
                        "optional": True,
                        "restricted-to": [(0, 255)],
                        "type": "INTEGER",
                    },
                    {"name": "carrierFreq", "type": "EARFCN"},
                    {
                        "actual-parameters": [
                            {"object-set": ["MBSFN-ResultToLogInfo-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MDT-Activation": {
                "type": "ENUMERATED",
                "values": [
                    ("immediate-MDT-only", 0),
                    ("immediate-MDT-and-Trace", 1),
                    ("logged-MDT-only", 2),
                    None,
                    ("logged-MBSFN-MDT", 3),
                ],
            },
            "MDT-Configuration": {
                "members": [
                    {"name": "mdt-Activation", "type": "MDT-Activation"},
                    {"name": "areaScopeOfMDT", "type": "AreaScopeOfMDT"},
                    {"name": "mDTMode", "type": "MDTMode"},
                    {
                        "actual-parameters": [
                            {"object-set": ["MDT-Configuration-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MDT-Location-Info": {"size": [8], "type": "BIT STRING"},
            "MDTMode": {
                "members": [
                    {"name": "immediateMDT", "type": "ImmediateMDT"},
                    {"name": "loggedMDT", "type": "LoggedMDT"},
                    None,
                    {"name": "mDTMode-Extension", "type": "MDTMode-Extension"},
                ],
                "type": "CHOICE",
            },
            "MDTMode-Extension": {
                "actual-parameters": [{"object-set": ["MDTMode-ExtensionIE"]}],
                "type": "ProtocolIE-SingleContainer",
            },
            "MDTPLMNList": {
                "element": {"type": "PLMNidentity"},
                "size": [(1, "maxnoofMDTPLMNs")],
                "type": "SEQUENCE OF",
            },
            "MME-Code": {"size": [1], "type": "OCTET STRING"},
            "MME-Group-ID": {"size": [2], "type": "OCTET STRING"},
            "MME-UE-S1AP-ID": {"restricted-to": [(0, 4294967295)], "type": "INTEGER"},
            "MMEPagingTarget": {
                "members": [
                    {"name": "global-ENB-ID", "type": "Global-ENB-ID"},
                    {"name": "tAI", "type": "TAI"},
                    None,
                ],
                "type": "CHOICE",
            },
            "MMERelaySupportIndicator": {
                "type": "ENUMERATED",
                "values": [("true", 0), None],
            },
            "MMEname": {"size": [(1, 150), None], "type": "PrintableString"},
            "MSClassmark2": {"type": "OCTET STRING"},
            "MSClassmark3": {"type": "OCTET STRING"},
            "ManagementBasedMDTAllowed": {
                "type": "ENUMERATED",
                "values": [("allowed", 0), None],
            },
            "Masked-IMEISV": {"size": [64], "type": "BIT STRING"},
            "MeasurementThresholdA2": {
                "members": [
                    {"name": "threshold-RSRP", "type": "Threshold-RSRP"},
                    {"name": "threshold-RSRQ", "type": "Threshold-RSRQ"},
                    None,
                ],
                "type": "CHOICE",
            },
            "MeasurementsToActivate": {"size": [8], "type": "BIT STRING"},
            "MessageIdentifier": {"size": [16], "type": "BIT STRING"},
            "MobilityInformation": {"size": [32], "type": "BIT STRING"},
            "MutingAvailabilityIndication": {
                "type": "ENUMERATED",
                "values": [("available", 0), ("unavailable", 1), None],
            },
            "MutingPatternInformation": {
                "members": [
                    {
                        "name": "muting-pattern-period",
                        "type": "ENUMERATED",
                        "values": [
                            ("ms0", 0),
                            ("ms1280", 1),
                            ("ms2560", 2),
                            ("ms5120", 3),
                            ("ms10240", 4),
                            None,
                        ],
                    },
                    {
                        "name": "muting-pattern-offset",
                        "optional": True,
                        "restricted-to": [(0, 10239), None],
                        "type": "INTEGER",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["MutingPatternInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "NAS-PDU": {"type": "OCTET STRING"},
            "NASSecurityParametersfromE-UTRAN": {"type": "OCTET STRING"},
            "NASSecurityParameterstoE-UTRAN": {"type": "OCTET STRING"},
            "NB-IoT-DefaultPagingDRX": {
                "type": "ENUMERATED",
                "values": [("v128", 0), ("v256", 1), ("v512", 2), ("v1024", 3), None],
            },
            "NB-IoT-Paging-eDRX-Cycle": {
                "type": "ENUMERATED",
                "values": [
                    ("hf2", 0),
                    ("hf4", 1),
                    ("hf6", 2),
                    ("hf8", 3),
                    ("hf10", 4),
                    ("hf12", 5),
                    ("hf14", 6),
                    ("hf16", 7),
                    ("hf32", 8),
                    ("hf64", 9),
                    ("hf128", 10),
                    ("hf256", 11),
                    ("hf512", 12),
                    ("hf1024", 13),
                    None,
                ],
            },
            "NB-IoT-Paging-eDRXInformation": {
                "members": [
                    {
                        "name": "nB-IoT-paging-eDRX-Cycle",
                        "type": "NB-IoT-Paging-eDRX-Cycle",
                    },
                    {
                        "name": "nB-IoT-pagingTimeWindow",
                        "optional": True,
                        "type": "NB-IoT-PagingTimeWindow",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["NB-IoT-Paging-eDRXInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "NB-IoT-PagingTimeWindow": {
                "type": "ENUMERATED",
                "values": [
                    ("s1", 0),
                    ("s2", 1),
                    ("s3", 2),
                    ("s4", 3),
                    ("s5", 4),
                    ("s6", 5),
                    ("s7", 6),
                    ("s8", 7),
                    ("s9", 8),
                    ("s10", 9),
                    ("s11", 10),
                    ("s12", 11),
                    ("s13", 12),
                    ("s14", 13),
                    ("s15", 14),
                    ("s16", 15),
                    None,
                ],
            },
            "NB-IoT-UEIdentityIndexValue": {"size": [12], "type": "BIT STRING"},
            "NextPagingAreaScope": {
                "type": "ENUMERATED",
                "values": [("same", 0), ("changed", 1), None],
            },
            "NumberOfBroadcasts": {"restricted-to": [(0, 65535)], "type": "INTEGER"},
            "NumberofBroadcastRequest": {
                "restricted-to": [(0, 65535)],
                "type": "INTEGER",
            },
            "OldBSS-ToNewBSS-Information": {"type": "OCTET STRING"},
            "OverloadAction": {
                "type": "ENUMERATED",
                "values": [
                    ("reject-non-emergency-mo-dt", 0),
                    ("reject-rrc-cr-signalling", 1),
                    (
                        "permit-emergency-sessions-and-mobile-terminated-services-only",
                        2,
                    ),
                    None,
                    (
                        "permit-high-priority-sessions-and-mobile-terminated-services-only",
                        3,
                    ),
                    ("reject-delay-tolerant-access", 4),
                    (
                        "permit-high-priority-sessions-and-exception-reporting-and-mobile-terminated-services-only",
                        5,
                    ),
                    ("not-accept-mo-data-or-delay-tolerant-access-from-CP-CIoT", 6),
                ],
            },
            "OverloadResponse": {
                "members": [{"name": "overloadAction", "type": "OverloadAction"}, None],
                "type": "CHOICE",
            },
            "PDCP-SN": {"restricted-to": [(0, 4095)], "type": "INTEGER"},
            "PDCP-SNExtended": {"restricted-to": [(0, 32767)], "type": "INTEGER"},
            "PDCP-SNlength18": {"restricted-to": [(0, 262143)], "type": "INTEGER"},
            "PLMNidentity": {"type": "TBCD-STRING"},
            "PS-ServiceNotAvailable": {
                "type": "ENUMERATED",
                "values": [("ps-service-not-available", 0), None],
            },
            "PWSfailedECGIList": {
                "element": {"type": "EUTRAN-CGI"},
                "size": [(1, "maxnoofCellsineNB")],
                "type": "SEQUENCE OF",
            },
            "Paging-eDRX-Cycle": {
                "type": "ENUMERATED",
                "values": [
                    ("hfhalf", 0),
                    ("hf1", 1),
                    ("hf2", 2),
                    ("hf4", 3),
                    ("hf6", 4),
                    ("hf8", 5),
                    ("hf10", 6),
                    ("hf12", 7),
                    ("hf14", 8),
                    ("hf16", 9),
                    ("hf32", 10),
                    ("hf64", 11),
                    ("hf128", 12),
                    ("hf256", 13),
                    None,
                ],
            },
            "Paging-eDRXInformation": {
                "members": [
                    {"name": "paging-eDRX-Cycle", "type": "Paging-eDRX-Cycle"},
                    {
                        "name": "pagingTimeWindow",
                        "optional": True,
                        "type": "PagingTimeWindow",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["Paging-eDRXInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PagingAttemptCount": {"restricted-to": [(1, 16), None], "type": "INTEGER"},
            "PagingAttemptInformation": {
                "members": [
                    {"name": "pagingAttemptCount", "type": "PagingAttemptCount"},
                    {
                        "name": "intendedNumberOfPagingAttempts",
                        "type": "IntendedNumberOfPagingAttempts",
                    },
                    {
                        "name": "nextPagingAreaScope",
                        "optional": True,
                        "type": "NextPagingAreaScope",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["PagingAttemptInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PagingDRX": {
                "type": "ENUMERATED",
                "values": [("v32", 0), ("v64", 1), ("v128", 2), ("v256", 3), None],
            },
            "PagingPriority": {
                "type": "ENUMERATED",
                "values": [
                    ("priolevel1", 0),
                    ("priolevel2", 1),
                    ("priolevel3", 2),
                    ("priolevel4", 3),
                    ("priolevel5", 4),
                    ("priolevel6", 5),
                    ("priolevel7", 6),
                    ("priolevel8", 7),
                    None,
                ],
            },
            "PagingTimeWindow": {
                "type": "ENUMERATED",
                "values": [
                    ("s1", 0),
                    ("s2", 1),
                    ("s3", 2),
                    ("s4", 3),
                    ("s5", 4),
                    ("s6", 5),
                    ("s7", 6),
                    ("s8", 7),
                    ("s9", 8),
                    ("s10", 9),
                    ("s11", 10),
                    ("s12", 11),
                    ("s13", 12),
                    ("s14", 13),
                    ("s15", 14),
                    ("s16", 15),
                    None,
                ],
            },
            "PedestrianUE": {
                "type": "ENUMERATED",
                "values": [("authorized", 0), ("not-authorized", 1), None],
            },
            "Port-Number": {"size": [2], "type": "OCTET STRING"},
            "Pre-emptionCapability": {
                "type": "ENUMERATED",
                "values": [
                    ("shall-not-trigger-pre-emption", 0),
                    ("may-trigger-pre-emption", 1),
                ],
            },
            "Pre-emptionVulnerability": {
                "type": "ENUMERATED",
                "values": [("not-pre-emptable", 0), ("pre-emptable", 1)],
            },
            "PriorityLevel": {
                "named-numbers": {
                    "highest": 1,
                    "lowest": 14,
                    "no-priority": 15,
                    "spare": 0,
                },
                "restricted-to": [(0, 15)],
                "type": "INTEGER",
            },
            "PrivacyIndicator": {
                "type": "ENUMERATED",
                "values": [("immediate-MDT", 0), ("logged-MDT", 1), None],
            },
            "ProSeAuthorized": {
                "members": [
                    {
                        "name": "proSeDirectDiscovery",
                        "optional": True,
                        "type": "ProSeDirectDiscovery",
                    },
                    {
                        "name": "proSeDirectCommunication",
                        "optional": True,
                        "type": "ProSeDirectCommunication",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["ProSeAuthorized-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ProSeDirectCommunication": {
                "type": "ENUMERATED",
                "values": [("authorized", 0), ("not-authorized", 1), None],
            },
            "ProSeDirectDiscovery": {
                "type": "ENUMERATED",
                "values": [("authorized", 0), ("not-authorized", 1), None],
            },
            "ProSeUEtoNetworkRelaying": {
                "type": "ENUMERATED",
                "values": [("authorized", 0), ("not-authorized", 1), None],
            },
            "QCI": {"restricted-to": [(0, 255)], "type": "INTEGER"},
            "RAC": {"size": [1], "type": "OCTET STRING"},
            "RAT-Type": {"type": "ENUMERATED", "values": [("nbiot", 0), None]},
            "RIMInformation": {"type": "OCTET STRING"},
            "RIMRoutingAddress": {
                "members": [
                    {"name": "gERAN-Cell-ID", "type": "GERAN-Cell-ID"},
                    None,
                    {"name": "targetRNC-ID", "type": "TargetRNC-ID"},
                    {"name": "eHRPD-Sector-ID", "size": [16], "type": "OCTET STRING"},
                ],
                "type": "CHOICE",
            },
            "RIMTransfer": {
                "members": [
                    {"name": "rIMInformation", "type": "RIMInformation"},
                    {
                        "name": "rIMRoutingAddress",
                        "optional": True,
                        "type": "RIMRoutingAddress",
                    },
                    {
                        "actual-parameters": [{"object-set": ["RIMTransfer-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RLFReportInformation": {
                "members": [
                    {
                        "name": "uE-RLF-Report-Container",
                        "type": "UE-RLF-Report-Container",
                    },
                    {
                        "name": "uE-RLF-Report-Container-for-extended-bands",
                        "optional": True,
                        "type": "UE-RLF-Report-Container-for-extended-bands",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["RLFReportInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RNC-ID": {"restricted-to": [(0, 4095)], "type": "INTEGER"},
            "RRC-Container": {"type": "OCTET STRING"},
            "RRC-Establishment-Cause": {
                "type": "ENUMERATED",
                "values": [
                    ("emergency", 0),
                    ("highPriorityAccess", 1),
                    ("mt-Access", 2),
                    ("mo-Signalling", 3),
                    ("mo-Data", 4),
                    None,
                    ("delay-TolerantAccess", 5),
                    ("mo-VoiceCall", 6),
                    ("mo-ExceptionData", 7),
                ],
            },
            "ReceiveStatusOfULPDCPSDUsExtended": {
                "size": [(1, 16384)],
                "type": "BIT STRING",
            },
            "ReceiveStatusOfULPDCPSDUsPDCP-SNlength18": {
                "size": [(1, 131072)],
                "type": "BIT STRING",
            },
            "ReceiveStatusofULPDCPSDUs": {"size": [4096], "type": "BIT STRING"},
            "RecommendedCellItem": {
                "members": [
                    {"name": "eUTRAN-CGI", "type": "EUTRAN-CGI"},
                    {
                        "name": "timeStayedInCell",
                        "optional": True,
                        "restricted-to": [(0, 4095)],
                        "type": "INTEGER",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["RecommendedCellsForPagingItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RecommendedCellList": {
                "element": {
                    "actual-parameters": [{"object-set": ["RecommendedCellItemIEs"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofRecommendedCells")],
                "type": "SEQUENCE OF",
            },
            "RecommendedCellsForPaging": {
                "members": [
                    {"name": "recommendedCellList", "type": "RecommendedCellList"},
                    {
                        "actual-parameters": [
                            {"object-set": ["RecommendedCellsForPaging-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RecommendedENBItem": {
                "members": [
                    {"name": "mMEPagingTarget", "type": "MMEPagingTarget"},
                    {
                        "actual-parameters": [
                            {"object-set": ["RecommendedENBItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RecommendedENBList": {
                "element": {
                    "actual-parameters": [{"object-set": ["RecommendedENBItemIEs"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofRecommendedENBs")],
                "type": "SEQUENCE OF",
            },
            "RecommendedENBsForPaging": {
                "members": [
                    {"name": "recommendedENBList", "type": "RecommendedENBList"},
                    {
                        "actual-parameters": [
                            {"object-set": ["RecommendedENBsForPaging-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "RelativeMMECapacity": {"restricted-to": [(0, 255)], "type": "INTEGER"},
            "RelayNode-Indicator": {
                "type": "ENUMERATED",
                "values": [("true", 0), None],
            },
            "RepetitionPeriod": {"restricted-to": [(0, 4095)], "type": "INTEGER"},
            "ReportAmountMDT": {
                "type": "ENUMERATED",
                "values": [
                    ("r1", 0),
                    ("r2", 1),
                    ("r4", 2),
                    ("r8", 3),
                    ("r16", 4),
                    ("r32", 5),
                    ("r64", 6),
                    ("rinfinity", 7),
                ],
            },
            "ReportArea": {"type": "ENUMERATED", "values": [("ecgi", 0), None]},
            "ReportIntervalMDT": {
                "type": "ENUMERATED",
                "values": [
                    ("ms120", 0),
                    ("ms240", 1),
                    ("ms480", 2),
                    ("ms640", 3),
                    ("ms1024", 4),
                    ("ms2048", 5),
                    ("ms5120", 6),
                    ("ms10240", 7),
                    ("min1", 8),
                    ("min6", 9),
                    ("min12", 10),
                    ("min30", 11),
                    ("min60", 12),
                ],
            },
            "RequestType": {
                "members": [
                    {"name": "eventType", "type": "EventType"},
                    {"name": "reportArea", "type": "ReportArea"},
                    {
                        "actual-parameters": [{"object-set": ["RequestType-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Routing-ID": {"restricted-to": [(0, 255)], "type": "INTEGER"},
            "S-TMSI": {
                "members": [
                    {"name": "mMEC", "type": "MME-Code"},
                    {"name": "m-TMSI", "type": "M-TMSI"},
                    {
                        "actual-parameters": [{"object-set": ["S-TMSI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SONConfigurationTransfer": {
                "members": [
                    {"name": "targeteNB-ID", "type": "TargeteNB-ID"},
                    {"name": "sourceeNB-ID", "type": "SourceeNB-ID"},
                    {"name": "sONInformation", "type": "SONInformation"},
                    {
                        "actual-parameters": [
                            {"object-set": ["SONConfigurationTransfer-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SONInformation": {
                "members": [
                    {"name": "sONInformationRequest", "type": "SONInformationRequest"},
                    {"name": "sONInformationReply", "type": "SONInformationReply"},
                    None,
                    {
                        "name": "sONInformation-Extension",
                        "type": "SONInformation-Extension",
                    },
                ],
                "type": "CHOICE",
            },
            "SONInformation-Extension": {
                "actual-parameters": [{"object-set": ["SONInformation-ExtensionIE"]}],
                "type": "ProtocolIE-SingleContainer",
            },
            "SONInformationReply": {
                "members": [
                    {
                        "name": "x2TNLConfigurationInfo",
                        "optional": True,
                        "type": "X2TNLConfigurationInfo",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["SONInformationReply-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SONInformationReport": {
                "members": [
                    {"name": "rLFReportInformation", "type": "RLFReportInformation"},
                    None,
                ],
                "type": "CHOICE",
            },
            "SONInformationRequest": {
                "type": "ENUMERATED",
                "values": [
                    ("x2TNL-Configuration-Info", 0),
                    None,
                    ("time-Synchronisation-Info", 1),
                    ("activate-Muting", 2),
                    ("deactivate-Muting", 3),
                ],
            },
            "SRVCCHOIndication": {
                "type": "ENUMERATED",
                "values": [("pSandCS", 0), ("cSonly", 1), None],
            },
            "SRVCCOperationNotPossible": {
                "type": "ENUMERATED",
                "values": [("notPossible", 0), None],
            },
            "SRVCCOperationPossible": {
                "type": "ENUMERATED",
                "values": [("possible", 0), None],
            },
            "SecurityContext": {
                "members": [
                    {
                        "name": "nextHopChainingCount",
                        "restricted-to": [(0, 7)],
                        "type": "INTEGER",
                    },
                    {"name": "nextHopParameter", "type": "SecurityKey"},
                    {
                        "actual-parameters": [
                            {"object-set": ["SecurityContext-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SecurityKey": {"size": [256], "type": "BIT STRING"},
            "SerialNumber": {"size": [16], "type": "BIT STRING"},
            "ServedDCNs": {
                "element": {"type": "ServedDCNsItem"},
                "size": [(0, "maxnoofDCNs")],
                "type": "SEQUENCE OF",
            },
            "ServedDCNsItem": {
                "members": [
                    {"name": "dCN-ID", "type": "DCN-ID"},
                    {"name": "relativeDCNCapacity", "type": "RelativeMMECapacity"},
                    {
                        "actual-parameters": [
                            {"object-set": ["ServedDCNsItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ServedGUMMEIs": {
                "element": {"type": "ServedGUMMEIsItem"},
                "size": [(1, "maxnoofRATs")],
                "type": "SEQUENCE OF",
            },
            "ServedGUMMEIsItem": {
                "members": [
                    {"name": "servedPLMNs", "type": "ServedPLMNs"},
                    {"name": "servedGroupIDs", "type": "ServedGroupIDs"},
                    {"name": "servedMMECs", "type": "ServedMMECs"},
                    {
                        "actual-parameters": [
                            {"object-set": ["ServedGUMMEIsItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ServedGroupIDs": {
                "element": {"type": "MME-Group-ID"},
                "size": [(1, "maxnoofGroupIDs")],
                "type": "SEQUENCE OF",
            },
            "ServedMMECs": {
                "element": {"type": "MME-Code"},
                "size": [(1, "maxnoofMMECs")],
                "type": "SEQUENCE OF",
            },
            "ServedPLMNs": {
                "element": {"type": "PLMNidentity"},
                "size": [(1, "maxnoofPLMNsPerMME")],
                "type": "SEQUENCE OF",
            },
            "Source-ToTarget-TransparentContainer": {"type": "OCTET STRING"},
            "SourceBSS-ToTargetBSS-TransparentContainer": {"type": "OCTET STRING"},
            "SourceOfUEActivityBehaviourInformation": {
                "type": "ENUMERATED",
                "values": [("subscription-information", 0), ("statistics", 1), None],
            },
            "SourceRNC-ToTargetRNC-TransparentContainer": {"type": "OCTET STRING"},
            "SourceeNB-ID": {
                "members": [
                    {"name": "global-ENB-ID", "type": "Global-ENB-ID"},
                    {"name": "selected-TAI", "type": "TAI"},
                    {
                        "actual-parameters": [{"object-set": ["SourceeNB-ID-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                ],
                "type": "SEQUENCE",
            },
            "SourceeNB-ToTargeteNB-TransparentContainer": {
                "members": [
                    {"name": "rRC-Container", "type": "RRC-Container"},
                    {
                        "name": "e-RABInformationList",
                        "optional": True,
                        "type": "E-RABInformationList",
                    },
                    {"name": "targetCell-ID", "type": "EUTRAN-CGI"},
                    {
                        "name": "subscriberProfileIDforRFP",
                        "optional": True,
                        "type": "SubscriberProfileIDforRFP",
                    },
                    {"name": "uE-HistoryInformation", "type": "UE-HistoryInformation"},
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "SourceeNB-ToTargeteNB-TransparentContainer-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "StratumLevel": {"restricted-to": [(0, 3), None], "type": "INTEGER"},
            "SubscriberProfileIDforRFP": {
                "restricted-to": [(1, 256)],
                "type": "INTEGER",
            },
            "SupportedTAs": {
                "element": {"type": "SupportedTAs-Item"},
                "size": [(1, "maxnoofTACs")],
                "type": "SEQUENCE OF",
            },
            "SupportedTAs-Item": {
                "members": [
                    {"name": "tAC", "type": "TAC"},
                    {"name": "broadcastPLMNs", "type": "BPLMNs"},
                    {
                        "actual-parameters": [
                            {"object-set": ["SupportedTAs-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SynchronisationInformation": {
                "members": [
                    {
                        "name": "sourceStratumLevel",
                        "optional": True,
                        "type": "StratumLevel",
                    },
                    {
                        "name": "listeningSubframePattern",
                        "optional": True,
                        "type": "ListeningSubframePattern",
                    },
                    {
                        "name": "aggressoreCGI-List",
                        "optional": True,
                        "type": "ECGI-List",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["SynchronisationInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "SynchronisationStatus": {
                "type": "ENUMERATED",
                "values": [("synchronous", 0), ("asynchronous", 1), None],
            },
            "TABasedMDT": {
                "members": [
                    {"name": "tAListforMDT", "type": "TAListforMDT"},
                    {
                        "actual-parameters": [{"object-set": ["TABasedMDT-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAC": {"size": [2], "type": "OCTET STRING"},
            "TAI": {
                "members": [
                    {"name": "pLMNidentity", "type": "PLMNidentity"},
                    {"name": "tAC", "type": "TAC"},
                    {
                        "actual-parameters": [{"object-set": ["TAI-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAI-Broadcast": {
                "element": {"type": "TAI-Broadcast-Item"},
                "size": [(1, "maxnoofTAIforWarning")],
                "type": "SEQUENCE OF",
            },
            "TAI-Broadcast-Item": {
                "members": [
                    {"name": "tAI", "type": "TAI"},
                    {"name": "completedCellinTAI", "type": "CompletedCellinTAI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["TAI-Broadcast-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAI-Cancelled": {
                "element": {"type": "TAI-Cancelled-Item"},
                "size": [(1, "maxnoofTAIforWarning")],
                "type": "SEQUENCE OF",
            },
            "TAI-Cancelled-Item": {
                "members": [
                    {"name": "tAI", "type": "TAI"},
                    {"name": "cancelledCellinTAI", "type": "CancelledCellinTAI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["TAI-Cancelled-Item-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAIBasedMDT": {
                "members": [
                    {"name": "tAIListforMDT", "type": "TAIListforMDT"},
                    {
                        "actual-parameters": [{"object-set": ["TAIBasedMDT-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAIListForRestart": {
                "element": {"type": "TAI"},
                "size": [(1, "maxnoofRestartTAIs")],
                "type": "SEQUENCE OF",
            },
            "TAIListforMDT": {
                "element": {"type": "TAI"},
                "size": [(1, "maxnoofTAforMDT")],
                "type": "SEQUENCE OF",
            },
            "TAIListforWarning": {
                "element": {"type": "TAI"},
                "size": [(1, "maxnoofTAIforWarning")],
                "type": "SEQUENCE OF",
            },
            "TAListforMDT": {
                "element": {"type": "TAC"},
                "size": [(1, "maxnoofTAforMDT")],
                "type": "SEQUENCE OF",
            },
            "TBCD-STRING": {"size": [3], "type": "OCTET STRING"},
            "Target-ToSource-TransparentContainer": {"type": "OCTET STRING"},
            "TargetBSS-ToSourceBSS-TransparentContainer": {"type": "OCTET STRING"},
            "TargetID": {
                "members": [
                    {"name": "targeteNB-ID", "type": "TargeteNB-ID"},
                    {"name": "targetRNC-ID", "type": "TargetRNC-ID"},
                    {"name": "cGI", "type": "CGI"},
                    None,
                ],
                "type": "CHOICE",
            },
            "TargetRNC-ID": {
                "members": [
                    {"name": "lAI", "type": "LAI"},
                    {"name": "rAC", "optional": True, "type": "RAC"},
                    {"name": "rNC-ID", "type": "RNC-ID"},
                    {
                        "name": "extendedRNC-ID",
                        "optional": True,
                        "type": "ExtendedRNC-ID",
                    },
                    {
                        "actual-parameters": [{"object-set": ["TargetRNC-ID-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TargetRNC-ToSourceRNC-TransparentContainer": {"type": "OCTET STRING"},
            "TargeteNB-ID": {
                "members": [
                    {"name": "global-ENB-ID", "type": "Global-ENB-ID"},
                    {"name": "selected-TAI", "type": "TAI"},
                    {
                        "actual-parameters": [{"object-set": ["TargeteNB-ID-ExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TargeteNB-ToSourceeNB-TransparentContainer": {
                "members": [
                    {"name": "rRC-Container", "type": "RRC-Container"},
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "TargeteNB-ToSourceeNB-TransparentContainer-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Threshold-RSRP": {"restricted-to": [(0, 97)], "type": "INTEGER"},
            "Threshold-RSRQ": {"restricted-to": [(0, 34)], "type": "INTEGER"},
            "Time-UE-StayedInCell": {"restricted-to": [(0, 4095)], "type": "INTEGER"},
            "Time-UE-StayedInCell-EnhancedGranularity": {
                "restricted-to": [(0, 40950)],
                "type": "INTEGER",
            },
            "TimeSynchronisationInfo": {
                "members": [
                    {"name": "stratumLevel", "type": "StratumLevel"},
                    {"name": "synchronisationStatus", "type": "SynchronisationStatus"},
                    {
                        "actual-parameters": [
                            {"object-set": ["TimeSynchronisationInfo-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TimeToWait": {
                "type": "ENUMERATED",
                "values": [
                    ("v1s", 0),
                    ("v2s", 1),
                    ("v5s", 2),
                    ("v10s", 3),
                    ("v20s", 4),
                    ("v60s", 5),
                    None,
                ],
            },
            "TraceActivation": {
                "members": [
                    {"name": "e-UTRAN-Trace-ID", "type": "E-UTRAN-Trace-ID"},
                    {"name": "interfacesToTrace", "type": "InterfacesToTrace"},
                    {"name": "traceDepth", "type": "TraceDepth"},
                    {
                        "name": "traceCollectionEntityIPAddress",
                        "type": "TransportLayerAddress",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["TraceActivation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TraceDepth": {
                "type": "ENUMERATED",
                "values": [
                    ("minimum", 0),
                    ("medium", 1),
                    ("maximum", 2),
                    ("minimumWithoutVendorSpecificExtension", 3),
                    ("mediumWithoutVendorSpecificExtension", 4),
                    ("maximumWithoutVendorSpecificExtension", 5),
                    None,
                ],
            },
            "TrafficLoadReductionIndication": {
                "restricted-to": [(1, 99)],
                "type": "INTEGER",
            },
            "TransportInformation": {
                "members": [
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "uL-GTP-TEID", "type": "GTP-TEID"},
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TransportLayerAddress": {"size": [(1, 160), None], "type": "BIT STRING"},
            "TunnelInformation": {
                "members": [
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {
                        "name": "uDP-Port-Number",
                        "optional": True,
                        "type": "Port-Number",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["Tunnel-Information-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TypeOfError": {
                "type": "ENUMERATED",
                "values": [("not-understood", 0), ("missing", 1), None],
            },
            "UE-HistoryInformation": {
                "element": {"type": "LastVisitedCell-Item"},
                "size": [(1, "maxnoofCells")],
                "type": "SEQUENCE OF",
            },
            "UE-HistoryInformationFromTheUE": {"type": "OCTET STRING"},
            "UE-RLF-Report-Container": {"type": "OCTET STRING"},
            "UE-RLF-Report-Container-for-extended-bands": {"type": "OCTET STRING"},
            "UE-RetentionInformation": {
                "type": "ENUMERATED",
                "values": [("ues-retained", 0), None],
            },
            "UE-S1AP-ID-pair": {
                "members": [
                    {"name": "mME-UE-S1AP-ID", "type": "MME-UE-S1AP-ID"},
                    {"name": "eNB-UE-S1AP-ID", "type": "ENB-UE-S1AP-ID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["UE-S1AP-ID-pair-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UE-S1AP-IDs": {
                "members": [
                    {"name": "uE-S1AP-ID-pair", "type": "UE-S1AP-ID-pair"},
                    {"name": "mME-UE-S1AP-ID", "type": "MME-UE-S1AP-ID"},
                    None,
                ],
                "type": "CHOICE",
            },
            "UE-Usage-Type": {"restricted-to": [(0, 255)], "type": "INTEGER"},
            "UE-associatedLogicalS1-ConnectionItem": {
                "members": [
                    {
                        "name": "mME-UE-S1AP-ID",
                        "optional": True,
                        "type": "MME-UE-S1AP-ID",
                    },
                    {
                        "name": "eNB-UE-S1AP-ID",
                        "optional": True,
                        "type": "ENB-UE-S1AP-ID",
                    },
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "UE-associatedLogicalS1-ConnectionItemExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEAggregateMaximumBitrate": {
                "members": [
                    {"name": "uEaggregateMaximumBitRateDL", "type": "BitRate"},
                    {"name": "uEaggregateMaximumBitRateUL", "type": "BitRate"},
                    {
                        "actual-parameters": [
                            {"object-set": ["UEAggregate-MaximumBitrates-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEIdentityIndexValue": {"size": [10], "type": "BIT STRING"},
            "UEPagingID": {
                "members": [
                    {"name": "s-TMSI", "type": "S-TMSI"},
                    {"name": "iMSI", "type": "IMSI"},
                    None,
                ],
                "type": "CHOICE",
            },
            "UERadioCapability": {"type": "OCTET STRING"},
            "UERadioCapabilityForPaging": {"type": "OCTET STRING"},
            "UESecurityCapabilities": {
                "members": [
                    {"name": "encryptionAlgorithms", "type": "EncryptionAlgorithms"},
                    {
                        "name": "integrityProtectionAlgorithms",
                        "type": "IntegrityProtectionAlgorithms",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["UESecurityCapabilities-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UESidelinkAggregateMaximumBitrate": {
                "members": [
                    {"name": "uESidelinkAggregateMaximumBitRate", "type": "BitRate"},
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "UE-Sidelink-Aggregate-MaximumBitrates-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEUserPlaneCIoTSupportIndicator": {
                "type": "ENUMERATED",
                "values": [("supported", 0), None],
            },
            "UL-CP-SecurityInformation": {
                "members": [
                    {"name": "ul-NAS-MAC", "type": "UL-NAS-MAC"},
                    {"name": "ul-NAS-Count", "type": "UL-NAS-Count"},
                    {
                        "actual-parameters": [
                            {"object-set": ["UL-CP-SecurityInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UL-NAS-Count": {"size": [5], "type": "BIT STRING"},
            "UL-NAS-MAC": {"size": [16], "type": "BIT STRING"},
            "UserLocationInformation": {
                "members": [
                    {"name": "eutran-cgi", "type": "EUTRAN-CGI"},
                    {"name": "tai", "type": "TAI"},
                    {
                        "actual-parameters": [
                            {"object-set": ["UserLocationInformation-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "V2XServicesAuthorized": {
                "members": [
                    {"name": "vehicleUE", "optional": True, "type": "VehicleUE"},
                    {"name": "pedestrianUE", "optional": True, "type": "PedestrianUE"},
                    {
                        "actual-parameters": [
                            {"object-set": ["V2XServicesAuthorized-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "VehicleUE": {
                "type": "ENUMERATED",
                "values": [("authorized", 0), ("not-authorized", 1), None],
            },
            "VoiceSupportMatchIndicator": {
                "type": "ENUMERATED",
                "values": [("supported", 0), ("not-supported", 1), None],
            },
            "WarningAreaList": {
                "members": [
                    {"name": "cellIDList", "type": "ECGIList"},
                    {"name": "trackingAreaListforWarning", "type": "TAIListforWarning"},
                    {"name": "emergencyAreaIDList", "type": "EmergencyAreaIDList"},
                    None,
                ],
                "type": "CHOICE",
            },
            "WarningMessageContents": {"size": [(1, 9600)], "type": "OCTET STRING"},
            "WarningSecurityInfo": {"size": [50], "type": "OCTET STRING"},
            "WarningType": {"size": [2], "type": "OCTET STRING"},
            "X2TNLConfigurationInfo": {
                "members": [
                    {"name": "eNBX2TransportLayerAddresses", "type": "ENBX2TLAs"},
                    {
                        "actual-parameters": [
                            {"object-set": ["X2TNLConfigurationInfo-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
        },
        "values": {},
    },
    "S1AP-PDU-Contents": {
        "extensibility-implied": False,
        "imports": {
            "S1AP-Constants": [
                "id-Additional-GUTI",
                "id-AdditionalCSFallbackIndicator",
                "id-AssistanceDataForPaging",
                "id-BearerType",
                "id-BroadcastCancelledAreaList",
                "id-BroadcastCompletedAreaList",
                "id-CE-mode-B-SupportIndicator",
                "id-CNDomain",
                "id-CSFallbackIndicator",
                "id-CSG-Id",
                "id-CSG-IdList",
                "id-CSGMembershipInfo",
                "id-CSGMembershipStatus",
                "id-Cause",
                "id-CellAccessMode",
                "id-CellIdentifierAndCELevelForCECapableUEs",
                "id-ConcurrentWarningMessageIndicator",
                "id-Correlation-ID",
                "id-Coverage-Level",
                "id-CriticalityDiagnostics",
                "id-DCN-ID",
                "id-DL-CP-SecurityInformation",
                "id-DLNASPDUDeliveryAckRequest",
                "id-Data-Forwarding-Not-Possible",
                "id-DataCodingScheme",
                "id-DefaultPagingDRX",
                "id-Direct-Forwarding-Path-Availability",
                "id-E-RABAdmittedItem",
                "id-E-RABAdmittedList",
                "id-E-RABDataForwardingItem",
                "id-E-RABFailedToBeReleasedList",
                "id-E-RABFailedToModifyList",
                "id-E-RABFailedToModifyListBearerModConf",
                "id-E-RABFailedToReleaseList",
                "id-E-RABFailedToResumeItemResumeReq",
                "id-E-RABFailedToResumeItemResumeRes",
                "id-E-RABFailedToResumeListResumeReq",
                "id-E-RABFailedToResumeListResumeRes",
                "id-E-RABFailedToSetupListBearerSURes",
                "id-E-RABFailedToSetupListCtxtSURes",
                "id-E-RABFailedToSetupListHOReqAck",
                "id-E-RABFailedtoSetupItemHOReqAck",
                "id-E-RABModify",
                "id-E-RABModifyItemBearerModConf",
                "id-E-RABModifyItemBearerModRes",
                "id-E-RABModifyListBearerModConf",
                "id-E-RABModifyListBearerModRes",
                "id-E-RABNotToBeModifiedItemBearerModInd",
                "id-E-RABNotToBeModifiedListBearerModInd",
                "id-E-RABRelease",
                "id-E-RABReleaseIndication",
                "id-E-RABReleaseItemBearerRelComp",
                "id-E-RABReleaseItemHOCmd",
                "id-E-RABReleaseListBearerRelComp",
                "id-E-RABReleasedList",
                "id-E-RABSetup",
                "id-E-RABSetupItemBearerSURes",
                "id-E-RABSetupItemCtxtSURes",
                "id-E-RABSetupListBearerSURes",
                "id-E-RABSetupListCtxtSURes",
                "id-E-RABSubjecttoDataForwardingList",
                "id-E-RABToBeModifiedItemBearerModInd",
                "id-E-RABToBeModifiedItemBearerModReq",
                "id-E-RABToBeModifiedListBearerModInd",
                "id-E-RABToBeModifiedListBearerModReq",
                "id-E-RABToBeReleasedList",
                "id-E-RABToBeReleasedListBearerModConf",
                "id-E-RABToBeSetupItemBearerSUReq",
                "id-E-RABToBeSetupItemCtxtSUReq",
                "id-E-RABToBeSetupItemHOReq",
                "id-E-RABToBeSetupListBearerSUReq",
                "id-E-RABToBeSetupListCtxtSUReq",
                "id-E-RABToBeSetupListHOReq",
                "id-E-RABToBeSwitchedDLItem",
                "id-E-RABToBeSwitchedDLList",
                "id-E-RABToBeSwitchedULItem",
                "id-E-RABToBeSwitchedULList",
                "id-E-RABtoReleaseListHOCmd",
                "id-E-UTRAN-Trace-ID",
                "id-ECGIListForRestart",
                "id-EUTRAN-CGI",
                "id-EUTRANRoundTripDelayEstimationInfo",
                "id-EmergencyAreaIDListForRestart",
                "id-EnhancedCoverageRestricted",
                "id-ExpectedUEBehaviour",
                "id-ExtendedRepetitionPeriod",
                "id-GERANtoLTEHOInformationRes",
                "id-GUMMEI-ID",
                "id-GUMMEIList",
                "id-GUMMEIType",
                "id-GW-TransportLayerAddress",
                "id-GWContextReleaseIndication",
                "id-Global-ENB-ID",
                "id-HandoverRestrictionList",
                "id-HandoverType",
                "id-InformationOnRecommendedCellsAndENBsForPaging",
                "id-InitialContextSetup",
                "id-Inter-SystemInformationTransferTypeEDT",
                "id-Inter-SystemInformationTransferTypeMDT",
                "id-KillAllWarningMessages",
                "id-LHN-ID",
                "id-LPPa-PDU",
                "id-MME-Group-ID",
                "id-MME-UE-S1AP-ID",
                "id-MME-UE-S1AP-ID-2",
                "id-MMERelaySupportIndicator",
                "id-MMEname",
                "id-MSClassmark2",
                "id-MSClassmark3",
                "id-ManagementBasedMDTAllowed",
                "id-ManagementBasedMDTPLMNList",
                "id-Masked-IMEISV",
                "id-MessageIdentifier",
                "id-NAS-DownlinkCount",
                "id-NAS-PDU",
                "id-NASSecurityParametersfromE-UTRAN",
                "id-NASSecurityParameterstoE-UTRAN",
                "id-NB-IoT-DefaultPagingDRX",
                "id-NB-IoT-Paging-eDRXInformation",
                "id-NB-IoT-UEIdentityIndexValue",
                "id-NumberofBroadcastRequest",
                "id-OverloadResponse",
                "id-PS-ServiceNotAvailable",
                "id-PWSFailureIndication",
                "id-PWSfailedECGIList",
                "id-Paging-eDRXInformation",
                "id-PagingPriority",
                "id-PrivacyIndicator",
                "id-ProSeAuthorized",
                "id-RRC-Establishment-Cause",
                "id-RRC-Resume-Cause",
                "id-RegisteredLAI",
                "id-RelativeMMECapacity",
                "id-RelayNode-Indicator",
                "id-RepetitionPeriod",
                "id-RequestType",
                "id-ResetType",
                "id-Routing-ID",
                "id-S-TMSI",
                "id-S1-Message",
                "id-SIPTO-Correlation-ID",
                "id-SIPTO-L-GW-TransportLayerAddress",
                "id-SONConfigurationTransferECT",
                "id-SONConfigurationTransferMCT",
                "id-SRVCCHOIndication",
                "id-SRVCCOperationNotPossible",
                "id-SRVCCOperationPossible",
                "id-SecurityContext",
                "id-SecurityKey",
                "id-SerialNumber",
                "id-ServedDCNs",
                "id-ServedGUMMEIs",
                "id-Source-ToTarget-TransparentContainer",
                "id-Source-ToTarget-TransparentContainer-Secondary",
                "id-SourceMME-GUMMEI",
                "id-SourceMME-UE-S1AP-ID",
                "id-SubscriberProfileIDforRFP",
                "id-SupportedTAs",
                "id-TAI",
                "id-TAIItem",
                "id-TAIList",
                "id-TAIListForRestart",
                "id-Target-ToSource-TransparentContainer",
                "id-Target-ToSource-TransparentContainer-Secondary",
                "id-TargetID",
                "id-TimeToWait",
                "id-TraceActivation",
                "id-TraceCollectionEntityIPAddress",
                "id-TrafficLoadReductionIndication",
                "id-TransportInformation",
                "id-Tunnel-Information-for-BBF",
                "id-UE-Level-QoS-Parameters",
                "id-UE-RetentionInformation",
                "id-UE-S1AP-IDs",
                "id-UE-Usage-Type",
                "id-UE-associatedLogicalS1-ConnectionItem",
                "id-UE-associatedLogicalS1-ConnectionListResAck",
                "id-UEIdentityIndexValue",
                "id-UEPagingID",
                "id-UERadioCapability",
                "id-UERadioCapabilityForPaging",
                "id-UESecurityCapabilities",
                "id-UESidelinkAggregateMaximumBitrate",
                "id-UEUserPlaneCIoTSupportIndicator",
                "id-UL-CP-SecurityInformation",
                "id-UTRANtoLTEHOInformationRes",
                "id-UserLocationInformation",
                "id-V2XServicesAuthorized",
                "id-VoiceSupportMatchIndicator",
                "id-WarningAreaList",
                "id-WarningMessageContents",
                "id-WarningSecurityInfo",
                "id-WarningType",
                "id-cdma2000HORequiredIndication",
                "id-cdma2000HOStatus",
                "id-cdma2000OneXRAND",
                "id-cdma2000OneXSRVCCInfo",
                "id-cdma2000PDU",
                "id-cdma2000RATType",
                "id-cdma2000SectorID",
                "id-eNB-StatusTransfer-TransparentContainer",
                "id-eNB-UE-S1AP-ID",
                "id-eNBname",
                "id-extended-UEIdentityIndexValue",
                "id-pagingDRX",
                "id-uEaggregateMaximumBitrate",
                "maxnoofCellID",
                "maxnoofCellinEAI",
                "maxnoofCellinTAI",
                "maxnoofE-RABs",
                "maxnoofEmergencyAreaID",
                "maxnoofErrors",
                "maxnoofIndividualS1ConnectionsToReset",
                "maxnoofTAIforWarning",
                "maxnoofTAIs",
            ],
            "S1AP-Containers": [
                "PrivateIE-Container",
                "ProtocolExtensionContainer",
                "ProtocolIE-Container",
                "ProtocolIE-ContainerList",
                "ProtocolIE-ContainerPair",
                "ProtocolIE-ContainerPairList",
                "ProtocolIE-SingleContainer",
                "S1AP-PRIVATE-IES",
                "S1AP-PROTOCOL-EXTENSION",
                "S1AP-PROTOCOL-IES",
                "S1AP-PROTOCOL-IES-PAIR",
                "{",
                "{",
                "{",
                "{",
                "{",
                "{",
                "{",
                "}",
                "}",
                "}",
                "}",
                "}",
                "}",
                "}",
            ],
            "S1AP-IEs": [
                "Additional-GUTI",
                "AdditionalCSFallbackIndicator",
                "AssistanceDataForPaging",
                "BearerType",
                "BroadcastCancelledAreaList",
                "BroadcastCompletedAreaList",
                "CE-mode-B-SupportIndicator",
                "CNDomain",
                "CSFallbackIndicator",
                "CSG-Id",
                "CSG-IdList",
                "CSGMembershipStatus",
                "Cause",
                "Cdma2000HORequiredIndication",
                "Cdma2000HOStatus",
                "Cdma2000OneXRAND",
                "Cdma2000OneXSRVCCInfo",
                "Cdma2000PDU",
                "Cdma2000RATType",
                "Cdma2000SectorID",
                "CellAccessMode",
                "CellIdentifierAndCELevelForCECapableUEs",
                "ConcurrentWarningMessageIndicator",
                "Correlation-ID",
                "Coverage-Level",
                "CriticalityDiagnostics",
                "DCN-ID",
                "DL-CP-SecurityInformation",
                "DLNASPDUDeliveryAckRequest",
                "Data-Forwarding-Not-Possible",
                "DataCodingScheme",
                "Direct-Forwarding-Path-Availability",
                "E-RAB-ID",
                "E-RABLevelQoSParameters",
                "E-RABList",
                "E-UTRAN-Trace-ID",
                "ECGIListForRestart",
                "ENB-StatusTransfer-TransparentContainer",
                "ENB-UE-S1AP-ID",
                "ENBname",
                "EUTRAN-CGI",
                "EUTRANRoundTripDelayEstimationInfo",
                "EmergencyAreaIDListForRestart",
                "EnhancedCoverageRestricted",
                "ExpectedUEBehaviour",
                "Extended-UEIdentityIndexValue",
                "ExtendedRepetitionPeriod",
                "GTP-TEID",
                "GUMMEI",
                "GUMMEIList",
                "GUMMEIType",
                "GWContextReleaseIndication",
                "Global-ENB-ID",
                "HandoverRestrictionList",
                "HandoverType",
                "InformationOnRecommendedCellsAndENBsForPaging",
                "KillAllWarningMessages",
                "LAI",
                "LHN-ID",
                "LPPa-PDU",
                "MDTPLMNList",
                "MME-Group-ID",
                "MME-UE-S1AP-ID",
                "MMERelaySupportIndicator",
                "MMEname",
                "MSClassmark2",
                "MSClassmark3",
                "ManagementBasedMDTAllowed",
                "Masked-IMEISV",
                "MessageIdentifier",
                "NAS-PDU",
                "NASSecurityParametersfromE-UTRAN",
                "NASSecurityParameterstoE-UTRAN",
                "NB-IoT-DefaultPagingDRX",
                "NB-IoT-Paging-eDRXInformation",
                "NB-IoT-UEIdentityIndexValue",
                "NumberofBroadcastRequest",
                "OverloadResponse",
                "PLMNidentity",
                "PS-ServiceNotAvailable",
                "PWSfailedECGIList",
                "Paging-eDRXInformation",
                "PagingDRX",
                "PagingPriority",
                "PrivacyIndicator",
                "ProSeAuthorized",
                "RIMTransfer",
                "RRC-Establishment-Cause",
                "RelativeMMECapacity",
                "RelayNode-Indicator",
                "RepetitionPeriod",
                "RequestType",
                "Routing-ID",
                "S-TMSI",
                "SONConfigurationTransfer",
                "SRVCCHOIndication",
                "SRVCCOperationNotPossible",
                "SRVCCOperationPossible",
                "SecurityContext",
                "SecurityKey",
                "SerialNumber",
                "ServedDCNs",
                "ServedGUMMEIs",
                "Source-ToTarget-TransparentContainer",
                "SourceBSS-ToTargetBSS-TransparentContainer",
                "SourceRNC-ToTargetRNC-TransparentContainer",
                "SourceeNB-ToTargeteNB-TransparentContainer",
                "SubscriberProfileIDforRFP",
                "SupportedTAs",
                "TAI",
                "TAIListForRestart",
                "Target-ToSource-TransparentContainer",
                "TargetBSS-ToSourceBSS-TransparentContainer",
                "TargetID",
                "TargetRNC-ToSourceRNC-TransparentContainer",
                "TargeteNB-ToSourceeNB-TransparentContainer",
                "TimeToWait",
                "TraceActivation",
                "TrafficLoadReductionIndication",
                "TransportInformation",
                "TransportLayerAddress",
                "TunnelInformation",
                "UE-RetentionInformation",
                "UE-S1AP-IDs",
                "UE-Usage-Type",
                "UE-associatedLogicalS1-ConnectionItem",
                "UEAggregateMaximumBitrate",
                "UEIdentityIndexValue",
                "UEPagingID",
                "UERadioCapability",
                "UERadioCapabilityForPaging",
                "UESecurityCapabilities",
                "UESidelinkAggregateMaximumBitrate",
                "UEUserPlaneCIoTSupportIndicator",
                "UL-CP-SecurityInformation",
                "UserLocationInformation",
                "V2XServicesAuthorized",
                "VoiceSupportMatchIndicator",
                "WarningAreaList",
                "WarningMessageContents",
                "WarningSecurityInfo",
                "WarningType",
            ],
        },
        "object-classes": {},
        "object-sets": {
            "CSGMembershipInfo-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "CellTrafficTraceIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-UTRAN-Trace-ID"},
                        "&criticality": "ignore",
                        "&id": "id-E-UTRAN-Trace-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TransportLayerAddress"},
                        "&criticality": "ignore",
                        "&id": "id-TraceCollectionEntityIPAddress",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "PrivacyIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-PrivacyIndicator",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ConnectionEstablishmentIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "EnhancedCoverageRestricted"},
                        "&criticality": "ignore",
                        "&id": "id-EnhancedCoverageRestricted",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "DL-CP-SecurityInformation"},
                        "&criticality": "ignore",
                        "&id": "id-DL-CP-SecurityInformation",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "DeactivateTraceIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-UTRAN-Trace-ID"},
                        "&criticality": "ignore",
                        "&id": "id-E-UTRAN-Trace-ID",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "DownlinkNASTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NAS-PDU"},
                        "&criticality": "reject",
                        "&id": "id-NAS-PDU",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "HandoverRestrictionList"},
                        "&criticality": "ignore",
                        "&id": "id-HandoverRestrictionList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SubscriberProfileIDforRFP"},
                        "&criticality": "ignore",
                        "&id": "id-SubscriberProfileIDforRFP",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCOperationPossible"},
                        "&criticality": "ignore",
                        "&id": "id-SRVCCOperationPossible",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "DLNASPDUDeliveryAckRequest"},
                        "&criticality": "ignore",
                        "&id": "id-DLNASPDUDeliveryAckRequest",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "EnhancedCoverageRestricted"},
                        "&criticality": "ignore",
                        "&id": "id-EnhancedCoverageRestricted",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "DownlinkNonUEAssociatedLPPaTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Routing-ID"},
                        "&criticality": "reject",
                        "&id": "id-Routing-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "LPPa-PDU"},
                        "&criticality": "reject",
                        "&id": "id-LPPa-PDU",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "DownlinkS1cdma2000tunnellingIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABSubjecttoDataForwardingList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSubjecttoDataForwardingList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cdma2000HOStatus"},
                        "&criticality": "ignore",
                        "&id": "id-cdma2000HOStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cdma2000RATType"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000RATType",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cdma2000PDU"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000PDU",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "DownlinkUEAssociatedLPPaTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Routing-ID"},
                        "&criticality": "reject",
                        "&id": "id-Routing-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "LPPa-PDU"},
                        "&criticality": "reject",
                        "&id": "id-LPPa-PDU",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABAdmittedItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABAdmittedItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABAdmittedItem"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABAdmittedItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABDataForwardingItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABDataForwardingItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABDataForwardingItem"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABDataForwardingItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABFailedToResumeItemResumeReq-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABFailedToResumeItemResumeReqIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABFailedToResumeItemResumeReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABFailedToResumeItemResumeReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABFailedToResumeItemResumeRes-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABFailedToResumeItemResumeResIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABFailedToResumeItemResumeRes"},
                        "&criticality": "reject",
                        "&id": "id-E-RABFailedToResumeItemResumeRes",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABFailedToSetupItemHOReqAckExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABFailedtoSetupItemHOReqAckIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABFailedToSetupItemHOReqAck"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedtoSetupItemHOReqAck",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABModificationConfirmIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABModifyListBearerModConf"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABModifyListBearerModConf",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToModifyListBearerModConf",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABToBeReleasedListBearerModConf",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABModificationIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABToBeModifiedListBearerModInd"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeModifiedListBearerModInd",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABNotToBeModifiedListBearerModInd"},
                        "&criticality": "reject",
                        "&id": "id-E-RABNotToBeModifiedListBearerModInd",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipInfo"},
                        "&criticality": "reject",
                        "&id": "id-CSGMembershipInfo",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TunnelInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Tunnel-Information-for-BBF",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABModifyItemBearerModConfExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABModifyItemBearerModConfIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABModifyItemBearerModConf"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABModifyItemBearerModConf",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABModifyItemBearerModResExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABModifyItemBearerModResIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABModifyItemBearerModRes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABModifyItemBearerModRes",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABModifyRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "reject",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABToBeModifiedListBearerModReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeModifiedListBearerModReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABModifyResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABModifyListBearerModRes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABModifyListBearerModRes",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToModifyList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABNotToBeModifiedItemBearerModInd-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABNotToBeModifiedItemBearerModIndIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABNotToBeModifiedItemBearerModInd"},
                        "&criticality": "reject",
                        "&id": "id-E-RABNotToBeModifiedItemBearerModInd",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABReleaseCommandIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "reject",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABToBeReleasedList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NAS-PDU"},
                        "&criticality": "ignore",
                        "&id": "id-NAS-PDU",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABReleaseIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABReleasedList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UserLocationInformation"},
                        "&criticality": "ignore",
                        "&id": "id-UserLocationInformation",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABReleaseItemBearerRelCompExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABReleaseItemBearerRelCompIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABReleaseItemBearerRelComp"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABReleaseItemBearerRelComp",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABReleaseResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABReleaseListBearerRelComp"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABReleaseListBearerRelComp",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToReleaseList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UserLocationInformation"},
                        "&criticality": "ignore",
                        "&id": "id-UserLocationInformation",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABSetupItemBearerSUResExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABSetupItemBearerSUResIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABSetupItemBearerSURes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSetupItemBearerSURes",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABSetupItemCtxtSUResExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABSetupItemCtxtSUResIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABSetupItemCtxtSURes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSetupItemCtxtSURes",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABSetupRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "reject",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABToBeSetupListBearerSUReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupListBearerSUReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABSetupResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABSetupListBearerSURes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSetupListBearerSURes",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToSetupListBearerSURes",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABToBeModifiedItemBearerModInd-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABToBeModifiedItemBearerModIndIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeModifiedItemBearerModInd"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeModifiedItemBearerModInd",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeModifiedItemBearerModReqIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeModifiedItemBearerModReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeModifiedItemBearerModReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeModifyItemBearerModReqExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "TransportInformation"},
                        "&criticality": "reject",
                        "&id": "id-TransportInformation",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemBearerSUReqExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "Correlation-ID"},
                        "&criticality": "ignore",
                        "&id": "id-Correlation-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "Correlation-ID"},
                        "&criticality": "ignore",
                        "&id": "id-SIPTO-Correlation-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "BearerType"},
                        "&criticality": "reject",
                        "&id": "id-BearerType",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemBearerSUReqIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeSetupItemBearerSUReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupItemBearerSUReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemCtxtSUReqExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "Correlation-ID"},
                        "&criticality": "ignore",
                        "&id": "id-Correlation-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "Correlation-ID"},
                        "&criticality": "ignore",
                        "&id": "id-SIPTO-Correlation-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "BearerType"},
                        "&criticality": "reject",
                        "&id": "id-BearerType",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemCtxtSUReqIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeSetupItemCtxtSUReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupItemCtxtSUReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemHOReq-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [
                    {
                        "&Extension": {"type": "Data-Forwarding-Not-Possible"},
                        "&criticality": "ignore",
                        "&id": "id-Data-Forwarding-Not-Possible",
                        "&presence": "optional",
                    },
                    {
                        "&Extension": {"type": "BearerType"},
                        "&criticality": "reject",
                        "&id": "id-BearerType",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "E-RABToBeSetupItemHOReqIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeSetupItemHOReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupItemHOReq",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeSwitchedDLItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABToBeSwitchedDLItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeSwitchedDLItem"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSwitchedDLItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "E-RABToBeSwitchedULItem-ExtIEs": {
                "class": "S1AP-PROTOCOL-EXTENSION",
                "members": [None],
            },
            "E-RABToBeSwitchedULItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "E-RABToBeSwitchedULItem"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABToBeSwitchedULItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "ENBCPRelocationIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "S-TMSI"},
                        "&criticality": "reject",
                        "&id": "id-S-TMSI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "ignore",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UL-CP-SecurityInformation"},
                        "&criticality": "reject",
                        "&id": "id-UL-CP-SecurityInformation",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "ENBConfigurationTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "SONConfigurationTransfer"},
                        "&criticality": "ignore",
                        "&id": "id-SONConfigurationTransferECT",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ENBConfigurationUpdateAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ENBConfigurationUpdateFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TimeToWait"},
                        "&criticality": "ignore",
                        "&id": "id-TimeToWait",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ENBConfigurationUpdateIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ENBname"},
                        "&criticality": "ignore",
                        "&id": "id-eNBname",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SupportedTAs"},
                        "&criticality": "reject",
                        "&id": "id-SupportedTAs",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSG-IdList"},
                        "&criticality": "reject",
                        "&id": "id-CSG-IdList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "PagingDRX"},
                        "&criticality": "ignore",
                        "&id": "id-DefaultPagingDRX",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "NB-IoT-DefaultPagingDRX"},
                        "&criticality": "ignore",
                        "&id": "id-NB-IoT-DefaultPagingDRX",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ENBDirectInformationTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Inter-SystemInformationTransferType"},
                        "&criticality": "reject",
                        "&id": "id-Inter-SystemInformationTransferTypeEDT",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "ENBStatusTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-StatusTransfer-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-eNB-StatusTransfer-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "ErrorIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverCancelAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverCancelIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "HandoverCommandIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "HandoverType"},
                        "&criticality": "reject",
                        "&id": "id-HandoverType",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NASSecurityParametersfromE-UTRAN"},
                        "&criticality": "reject",
                        "&id": "id-NASSecurityParametersfromE-UTRAN",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "E-RABSubjecttoDataForwardingList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSubjecttoDataForwardingList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABtoReleaseListHOCmd",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Target-ToSource-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Target-ToSource-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Target-ToSource-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Target-ToSource-TransparentContainer-Secondary",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverNotifyIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "ignore",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TunnelInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Tunnel-Information-for-BBF",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LHN-ID"},
                        "&criticality": "ignore",
                        "&id": "id-LHN-ID",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverPreparationFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverRequestAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABAdmittedList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABAdmittedList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABFailedtoSetupListHOReqAck"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToSetupListHOReqAck",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Target-ToSource-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Target-ToSource-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSG-Id"},
                        "&criticality": "ignore",
                        "&id": "id-CSG-Id",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellAccessMode"},
                        "&criticality": "ignore",
                        "&id": "id-CellAccessMode",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CE-mode-B-SupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-CE-mode-B-SupportIndicator",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "HandoverType"},
                        "&criticality": "reject",
                        "&id": "id-HandoverType",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "reject",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABToBeSetupListHOReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupListHOReq",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Source-ToTarget-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Source-ToTarget-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UESecurityCapabilities"},
                        "&criticality": "reject",
                        "&id": "id-UESecurityCapabilities",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "HandoverRestrictionList"},
                        "&criticality": "ignore",
                        "&id": "id-HandoverRestrictionList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TraceActivation"},
                        "&criticality": "ignore",
                        "&id": "id-TraceActivation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RequestType"},
                        "&criticality": "ignore",
                        "&id": "id-RequestType",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCOperationPossible"},
                        "&criticality": "ignore",
                        "&id": "id-SRVCCOperationPossible",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SecurityContext"},
                        "&criticality": "reject",
                        "&id": "id-SecurityContext",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NASSecurityParameterstoE-UTRAN"},
                        "&criticality": "reject",
                        "&id": "id-NASSecurityParameterstoE-UTRAN",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "CSG-Id"},
                        "&criticality": "reject",
                        "&id": "id-CSG-Id",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "GUMMEI"},
                        "&criticality": "ignore",
                        "&id": "id-GUMMEI-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID-2",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ManagementBasedMDTAllowed"},
                        "&criticality": "ignore",
                        "&id": "id-ManagementBasedMDTAllowed",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MDTPLMNList"},
                        "&criticality": "ignore",
                        "&id": "id-ManagementBasedMDTPLMNList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Masked-IMEISV"},
                        "&criticality": "ignore",
                        "&id": "id-Masked-IMEISV",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ExpectedUEBehaviour"},
                        "&criticality": "ignore",
                        "&id": "id-ExpectedUEBehaviour",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ProSeAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-ProSeAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UEUserPlaneCIoTSupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-UEUserPlaneCIoTSupportIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "V2XServicesAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-V2XServicesAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UESidelinkAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-UESidelinkAggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "EnhancedCoverageRestricted"},
                        "&criticality": "ignore",
                        "&id": "id-EnhancedCoverageRestricted",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "HandoverRequiredIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "HandoverType"},
                        "&criticality": "reject",
                        "&id": "id-HandoverType",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TargetID"},
                        "&criticality": "reject",
                        "&id": "id-TargetID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Direct-Forwarding-Path-Availability"},
                        "&criticality": "ignore",
                        "&id": "id-Direct-Forwarding-Path-Availability",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCHOIndication"},
                        "&criticality": "reject",
                        "&id": "id-SRVCCHOIndication",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Source-ToTarget-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Source-ToTarget-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Source-ToTarget-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-Source-ToTarget-TransparentContainer-Secondary",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MSClassmark2"},
                        "&criticality": "reject",
                        "&id": "id-MSClassmark2",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "MSClassmark3"},
                        "&criticality": "ignore",
                        "&id": "id-MSClassmark3",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "CSG-Id"},
                        "&criticality": "reject",
                        "&id": "id-CSG-Id",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellAccessMode"},
                        "&criticality": "reject",
                        "&id": "id-CellAccessMode",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "PS-ServiceNotAvailable"},
                        "&criticality": "ignore",
                        "&id": "id-PS-ServiceNotAvailable",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "InitialContextSetupFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "InitialContextSetupRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "reject",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABToBeSetupListCtxtSUReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSetupListCtxtSUReq",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UESecurityCapabilities"},
                        "&criticality": "reject",
                        "&id": "id-UESecurityCapabilities",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SecurityKey"},
                        "&criticality": "reject",
                        "&id": "id-SecurityKey",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TraceActivation"},
                        "&criticality": "ignore",
                        "&id": "id-TraceActivation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "HandoverRestrictionList"},
                        "&criticality": "ignore",
                        "&id": "id-HandoverRestrictionList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SubscriberProfileIDforRFP"},
                        "&criticality": "ignore",
                        "&id": "id-SubscriberProfileIDforRFP",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSFallbackIndicator"},
                        "&criticality": "reject",
                        "&id": "id-CSFallbackIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCOperationPossible"},
                        "&criticality": "ignore",
                        "&id": "id-SRVCCOperationPossible",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LAI"},
                        "&criticality": "ignore",
                        "&id": "id-RegisteredLAI",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "GUMMEI"},
                        "&criticality": "ignore",
                        "&id": "id-GUMMEI-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID-2",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ManagementBasedMDTAllowed"},
                        "&criticality": "ignore",
                        "&id": "id-ManagementBasedMDTAllowed",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MDTPLMNList"},
                        "&criticality": "ignore",
                        "&id": "id-ManagementBasedMDTPLMNList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "AdditionalCSFallbackIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-AdditionalCSFallbackIndicator",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "Masked-IMEISV"},
                        "&criticality": "ignore",
                        "&id": "id-Masked-IMEISV",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ExpectedUEBehaviour"},
                        "&criticality": "ignore",
                        "&id": "id-ExpectedUEBehaviour",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ProSeAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-ProSeAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UEUserPlaneCIoTSupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-UEUserPlaneCIoTSupportIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "V2XServicesAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-V2XServicesAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UESidelinkAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-UESidelinkAggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "EnhancedCoverageRestricted"},
                        "&criticality": "ignore",
                        "&id": "id-EnhancedCoverageRestricted",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "InitialContextSetupResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABSetupListCtxtSURes"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABSetupListCtxtSURes",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABFailedToSetupListCtxtSURes",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "InitialUEMessage-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NAS-PDU"},
                        "&criticality": "reject",
                        "&id": "id-NAS-PDU",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "reject",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "RRC-Establishment-Cause"},
                        "&criticality": "ignore",
                        "&id": "id-RRC-Establishment-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "S-TMSI"},
                        "&criticality": "reject",
                        "&id": "id-S-TMSI",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSG-Id"},
                        "&criticality": "reject",
                        "&id": "id-CSG-Id",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "GUMMEI"},
                        "&criticality": "reject",
                        "&id": "id-GUMMEI-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellAccessMode"},
                        "&criticality": "reject",
                        "&id": "id-CellAccessMode",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TransportLayerAddress"},
                        "&criticality": "ignore",
                        "&id": "id-GW-TransportLayerAddress",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RelayNode-Indicator"},
                        "&criticality": "reject",
                        "&id": "id-RelayNode-Indicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "GUMMEIType"},
                        "&criticality": "ignore",
                        "&id": "id-GUMMEIType",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TunnelInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Tunnel-Information-for-BBF",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TransportLayerAddress"},
                        "&criticality": "ignore",
                        "&id": "id-SIPTO-L-GW-TransportLayerAddress",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LHN-ID"},
                        "&criticality": "ignore",
                        "&id": "id-LHN-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MME-Group-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-Group-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UE-Usage-Type"},
                        "&criticality": "ignore",
                        "&id": "id-UE-Usage-Type",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CE-mode-B-SupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-CE-mode-B-SupportIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "DCN-ID"},
                        "&criticality": "ignore",
                        "&id": "id-DCN-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Coverage-Level"},
                        "&criticality": "ignore",
                        "&id": "id-Coverage-Level",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "KillRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MessageIdentifier"},
                        "&criticality": "reject",
                        "&id": "id-MessageIdentifier",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SerialNumber"},
                        "&criticality": "reject",
                        "&id": "id-SerialNumber",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "WarningAreaList"},
                        "&criticality": "ignore",
                        "&id": "id-WarningAreaList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "KillAllWarningMessages"},
                        "&criticality": "reject",
                        "&id": "id-KillAllWarningMessages",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "KillResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MessageIdentifier"},
                        "&criticality": "reject",
                        "&id": "id-MessageIdentifier",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SerialNumber"},
                        "&criticality": "reject",
                        "&id": "id-SerialNumber",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "BroadcastCancelledAreaList"},
                        "&criticality": "ignore",
                        "&id": "id-BroadcastCancelledAreaList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "LocationReportIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "ignore",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "RequestType"},
                        "&criticality": "ignore",
                        "&id": "id-RequestType",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "LocationReportingControlIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "RequestType"},
                        "&criticality": "ignore",
                        "&id": "id-RequestType",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "LocationReportingFailureIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "MMECPRelocationIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "MMEConfigurationTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "SONConfigurationTransfer"},
                        "&criticality": "ignore",
                        "&id": "id-SONConfigurationTransferMCT",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "MMEConfigurationUpdateAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "MMEConfigurationUpdateFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TimeToWait"},
                        "&criticality": "ignore",
                        "&id": "id-TimeToWait",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "MMEConfigurationUpdateIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MMEname"},
                        "&criticality": "ignore",
                        "&id": "id-MMEname",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ServedGUMMEIs"},
                        "&criticality": "reject",
                        "&id": "id-ServedGUMMEIs",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RelativeMMECapacity"},
                        "&criticality": "reject",
                        "&id": "id-RelativeMMECapacity",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ServedDCNs"},
                        "&criticality": "ignore",
                        "&id": "id-ServedDCNs",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "MMEDirectInformationTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Inter-SystemInformationTransferType"},
                        "&criticality": "reject",
                        "&id": "id-Inter-SystemInformationTransferTypeMDT",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "MMEStatusTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-StatusTransfer-TransparentContainer"},
                        "&criticality": "reject",
                        "&id": "id-eNB-StatusTransfer-TransparentContainer",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "NASDeliveryIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [None],
            },
            "NASNonDeliveryIndication-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NAS-PDU"},
                        "&criticality": "ignore",
                        "&id": "id-NAS-PDU",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "OverloadStartIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "OverloadResponse"},
                        "&criticality": "reject",
                        "&id": "id-OverloadResponse",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "GUMMEIList"},
                        "&criticality": "ignore",
                        "&id": "id-GUMMEIList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TrafficLoadReductionIndication"},
                        "&criticality": "ignore",
                        "&id": "id-TrafficLoadReductionIndication",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "OverloadStopIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "GUMMEIList"},
                        "&criticality": "ignore",
                        "&id": "id-GUMMEIList",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PWSFailureIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "PWSfailedECGIList"},
                        "&criticality": "reject",
                        "&id": "id-PWSfailedECGIList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Global-ENB-ID"},
                        "&criticality": "reject",
                        "&id": "id-Global-ENB-ID",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "PWSRestartIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ECGIListForRestart"},
                        "&criticality": "reject",
                        "&id": "id-ECGIListForRestart",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Global-ENB-ID"},
                        "&criticality": "reject",
                        "&id": "id-Global-ENB-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAIListForRestart"},
                        "&criticality": "reject",
                        "&id": "id-TAIListForRestart",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EmergencyAreaIDListForRestart"},
                        "&criticality": "reject",
                        "&id": "id-EmergencyAreaIDListForRestart",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PagingIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "UEIdentityIndexValue"},
                        "&criticality": "ignore",
                        "&id": "id-UEIdentityIndexValue",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEPagingID"},
                        "&criticality": "ignore",
                        "&id": "id-UEPagingID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "PagingDRX"},
                        "&criticality": "ignore",
                        "&id": "id-pagingDRX",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CNDomain"},
                        "&criticality": "ignore",
                        "&id": "id-CNDomain",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAIList"},
                        "&criticality": "ignore",
                        "&id": "id-TAIList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSG-IdList"},
                        "&criticality": "ignore",
                        "&id": "id-CSG-IdList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "PagingPriority"},
                        "&criticality": "ignore",
                        "&id": "id-PagingPriority",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UERadioCapabilityForPaging"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapabilityForPaging",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "AssistanceDataForPaging"},
                        "&criticality": "ignore",
                        "&id": "id-AssistanceDataForPaging",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Paging-eDRXInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Paging-eDRXInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Extended-UEIdentityIndexValue"},
                        "&criticality": "ignore",
                        "&id": "id-extended-UEIdentityIndexValue",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "NB-IoT-Paging-eDRXInformation"},
                        "&criticality": "ignore",
                        "&id": "id-NB-IoT-Paging-eDRXInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "NB-IoT-UEIdentityIndexValue"},
                        "&criticality": "ignore",
                        "&id": "id-NB-IoT-UEIdentityIndexValue",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PathSwitchRequestAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABToBeSwitchedULList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABToBeSwitchedULList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "E-RABList"},
                        "&criticality": "ignore",
                        "&id": "id-E-RABToBeReleasedList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SecurityContext"},
                        "&criticality": "reject",
                        "&id": "id-SecurityContext",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID-2",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ProSeAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-ProSeAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UEUserPlaneCIoTSupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-UEUserPlaneCIoTSupportIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "V2XServicesAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-V2XServicesAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UESidelinkAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-UESidelinkAggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "EnhancedCoverageRestricted"},
                        "&criticality": "ignore",
                        "&id": "id-EnhancedCoverageRestricted",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PathSwitchRequestFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PathSwitchRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABToBeSwitchedDLList"},
                        "&criticality": "reject",
                        "&id": "id-E-RABToBeSwitchedDLList",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-SourceMME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "ignore",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UESecurityCapabilities"},
                        "&criticality": "ignore",
                        "&id": "id-UESecurityCapabilities",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSG-Id"},
                        "&criticality": "ignore",
                        "&id": "id-CSG-Id",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellAccessMode"},
                        "&criticality": "ignore",
                        "&id": "id-CellAccessMode",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "GUMMEI"},
                        "&criticality": "ignore",
                        "&id": "id-SourceMME-GUMMEI",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TunnelInformation"},
                        "&criticality": "ignore",
                        "&id": "id-Tunnel-Information-for-BBF",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LHN-ID"},
                        "&criticality": "ignore",
                        "&id": "id-LHN-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RRC-Establishment-Cause"},
                        "&criticality": "ignore",
                        "&id": "id-RRC-Resume-Cause",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "PrivateMessageIEs": {"class": "S1AP-PRIVATE-IES", "members": [None]},
            "RerouteNASRequest-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "OCTET STRING"},
                        "&criticality": "reject",
                        "&id": "id-S1-Message",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "MME-Group-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-Group-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Additional-GUTI"},
                        "&criticality": "ignore",
                        "&id": "id-Additional-GUTI",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UE-Usage-Type"},
                        "&criticality": "ignore",
                        "&id": "id-UE-Usage-Type",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ResetAcknowledgeIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {
                            "type": "UE-associatedLogicalS1-ConnectionListResAck"
                        },
                        "&criticality": "ignore",
                        "&id": "id-UE-associatedLogicalS1-ConnectionListResAck",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "ResetIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ResetType"},
                        "&criticality": "reject",
                        "&id": "id-ResetType",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "RetrieveUEInformationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "S-TMSI"},
                        "&criticality": "reject",
                        "&id": "id-S-TMSI",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "S1SetupFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TimeToWait"},
                        "&criticality": "ignore",
                        "&id": "id-TimeToWait",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "S1SetupRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Global-ENB-ID"},
                        "&criticality": "reject",
                        "&id": "id-Global-ENB-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENBname"},
                        "&criticality": "ignore",
                        "&id": "id-eNBname",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SupportedTAs"},
                        "&criticality": "reject",
                        "&id": "id-SupportedTAs",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "PagingDRX"},
                        "&criticality": "ignore",
                        "&id": "id-DefaultPagingDRX",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSG-IdList"},
                        "&criticality": "reject",
                        "&id": "id-CSG-IdList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UE-RetentionInformation"},
                        "&criticality": "ignore",
                        "&id": "id-UE-RetentionInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "NB-IoT-DefaultPagingDRX"},
                        "&criticality": "ignore",
                        "&id": "id-NB-IoT-DefaultPagingDRX",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "S1SetupResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MMEname"},
                        "&criticality": "ignore",
                        "&id": "id-MMEname",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ServedGUMMEIs"},
                        "&criticality": "reject",
                        "&id": "id-ServedGUMMEIs",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "RelativeMMECapacity"},
                        "&criticality": "ignore",
                        "&id": "id-RelativeMMECapacity",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "MMERelaySupportIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-MMERelaySupportIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UE-RetentionInformation"},
                        "&criticality": "ignore",
                        "&id": "id-UE-RetentionInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ServedDCNs"},
                        "&criticality": "ignore",
                        "&id": "id-ServedDCNs",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "TAIItemExtIEs": {"class": "S1AP-PROTOCOL-EXTENSION", "members": [None]},
            "TAIItemIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "TAIItem"},
                        "&criticality": "ignore",
                        "&id": "id-TAIItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "TraceFailureIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-UTRAN-Trace-ID"},
                        "&criticality": "ignore",
                        "&id": "id-E-UTRAN-Trace-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "TraceStartIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TraceActivation"},
                        "&criticality": "ignore",
                        "&id": "id-TraceActivation",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "UE-associatedLogicalS1-ConnectionItemRes": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "UE-associatedLogicalS1-ConnectionItem"},
                        "&criticality": "reject",
                        "&id": "id-UE-associatedLogicalS1-ConnectionItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "UE-associatedLogicalS1-ConnectionItemResAck": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "UE-associatedLogicalS1-ConnectionItem"},
                        "&criticality": "ignore",
                        "&id": "id-UE-associatedLogicalS1-ConnectionItem",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "UECapabilityInfoIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UERadioCapabilityForPaging"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapabilityForPaging",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextModificationConfirmIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextModificationFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextModificationIndicationIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CSGMembershipInfo"},
                        "&criticality": "reject",
                        "&id": "id-CSGMembershipInfo",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextModificationRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SecurityKey"},
                        "&criticality": "reject",
                        "&id": "id-SecurityKey",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SubscriberProfileIDforRFP"},
                        "&criticality": "ignore",
                        "&id": "id-SubscriberProfileIDforRFP",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UEAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-uEaggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSFallbackIndicator"},
                        "&criticality": "reject",
                        "&id": "id-CSFallbackIndicator",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UESecurityCapabilities"},
                        "&criticality": "reject",
                        "&id": "id-UESecurityCapabilities",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CSGMembershipStatus"},
                        "&criticality": "ignore",
                        "&id": "id-CSGMembershipStatus",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LAI"},
                        "&criticality": "ignore",
                        "&id": "id-RegisteredLAI",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "AdditionalCSFallbackIndicator"},
                        "&criticality": "ignore",
                        "&id": "id-AdditionalCSFallbackIndicator",
                        "&presence": "conditional",
                    },
                    {
                        "&Value": {"type": "ProSeAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-ProSeAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCOperationPossible"},
                        "&criticality": "ignore",
                        "&id": "id-SRVCCOperationPossible",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SRVCCOperationNotPossible"},
                        "&criticality": "ignore",
                        "&id": "id-SRVCCOperationNotPossible",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "V2XServicesAuthorized"},
                        "&criticality": "ignore",
                        "&id": "id-V2XServicesAuthorized",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UESidelinkAggregateMaximumBitrate"},
                        "&criticality": "ignore",
                        "&id": "id-UESidelinkAggregateMaximumBitrate",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextModificationResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextReleaseCommand-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "UE-S1AP-IDs"},
                        "&criticality": "reject",
                        "&id": "id-UE-S1AP-IDs",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "UEContextReleaseComplete-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UserLocationInformation"},
                        "&criticality": "ignore",
                        "&id": "id-UserLocationInformation",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {
                            "type": "InformationOnRecommendedCellsAndENBsForPaging"
                        },
                        "&criticality": "ignore",
                        "&id": "id-InformationOnRecommendedCellsAndENBsForPaging",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellIdentifierAndCELevelForCECapableUEs"},
                        "&criticality": "ignore",
                        "&id": "id-CellIdentifierAndCELevelForCECapableUEs",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextReleaseRequest-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "GWContextReleaseIndication"},
                        "&criticality": "reject",
                        "&id": "id-GWContextReleaseIndication",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextResumeFailureIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cause"},
                        "&criticality": "ignore",
                        "&id": "id-Cause",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextResumeRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABFailedToResumeListResumeReq"},
                        "&criticality": "reject",
                        "&id": "id-E-RABFailedToResumeListResumeReq",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RRC-Establishment-Cause"},
                        "&criticality": "ignore",
                        "&id": "id-RRC-Resume-Cause",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextResumeResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABFailedToResumeListResumeRes"},
                        "&criticality": "reject",
                        "&id": "id-E-RABFailedToResumeListResumeRes",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SecurityContext"},
                        "&criticality": "reject",
                        "&id": "id-SecurityContext",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextSuspendRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {
                            "type": "InformationOnRecommendedCellsAndENBsForPaging"
                        },
                        "&criticality": "ignore",
                        "&id": "id-InformationOnRecommendedCellsAndENBsForPaging",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CellIdentifierAndCELevelForCECapableUEs"},
                        "&criticality": "ignore",
                        "&id": "id-CellIdentifierAndCELevelForCECapableUEs",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEContextSuspendResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "SecurityContext"},
                        "&criticality": "reject",
                        "&id": "id-SecurityContext",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UEInformationTransferIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "S-TMSI"},
                        "&criticality": "reject",
                        "&id": "id-S-TMSI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "E-RABLevelQoSParameters"},
                        "&criticality": "ignore",
                        "&id": "id-UE-Level-QoS-Parameters",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UERadioCapabilityMatchRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "UERadioCapability"},
                        "&criticality": "ignore",
                        "&id": "id-UERadioCapability",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UERadioCapabilityMatchResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "ignore",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "VoiceSupportMatchIndicator"},
                        "&criticality": "reject",
                        "&id": "id-VoiceSupportMatchIndicator",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UplinkNASTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "NAS-PDU"},
                        "&criticality": "reject",
                        "&id": "id-NAS-PDU",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRAN-CGI"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRAN-CGI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TAI"},
                        "&criticality": "ignore",
                        "&id": "id-TAI",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "TransportLayerAddress"},
                        "&criticality": "ignore",
                        "&id": "id-GW-TransportLayerAddress",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "TransportLayerAddress"},
                        "&criticality": "ignore",
                        "&id": "id-SIPTO-L-GW-TransportLayerAddress",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "LHN-ID"},
                        "&criticality": "ignore",
                        "&id": "id-LHN-ID",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UplinkNonUEAssociatedLPPaTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "Routing-ID"},
                        "&criticality": "reject",
                        "&id": "id-Routing-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "LPPa-PDU"},
                        "&criticality": "reject",
                        "&id": "id-LPPa-PDU",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "UplinkS1cdma2000tunnellingIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cdma2000RATType"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000RATType",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cdma2000SectorID"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000SectorID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Cdma2000HORequiredIndication"},
                        "&criticality": "ignore",
                        "&id": "id-cdma2000HORequiredIndication",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cdma2000OneXSRVCCInfo"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000OneXSRVCCInfo",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cdma2000OneXRAND"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000OneXRAND",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "Cdma2000PDU"},
                        "&criticality": "reject",
                        "&id": "id-cdma2000PDU",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "EUTRANRoundTripDelayEstimationInfo"},
                        "&criticality": "ignore",
                        "&id": "id-EUTRANRoundTripDelayEstimationInfo",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "UplinkUEAssociatedLPPaTransport-IEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MME-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-MME-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ENB-UE-S1AP-ID"},
                        "&criticality": "reject",
                        "&id": "id-eNB-UE-S1AP-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "Routing-ID"},
                        "&criticality": "reject",
                        "&id": "id-Routing-ID",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "LPPa-PDU"},
                        "&criticality": "reject",
                        "&id": "id-LPPa-PDU",
                        "&presence": "mandatory",
                    },
                    None,
                ],
            },
            "WriteReplaceWarningRequestIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MessageIdentifier"},
                        "&criticality": "reject",
                        "&id": "id-MessageIdentifier",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SerialNumber"},
                        "&criticality": "reject",
                        "&id": "id-SerialNumber",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "WarningAreaList"},
                        "&criticality": "ignore",
                        "&id": "id-WarningAreaList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "RepetitionPeriod"},
                        "&criticality": "reject",
                        "&id": "id-RepetitionPeriod",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "ExtendedRepetitionPeriod"},
                        "&criticality": "reject",
                        "&id": "id-ExtendedRepetitionPeriod",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "NumberofBroadcastRequest"},
                        "&criticality": "reject",
                        "&id": "id-NumberofBroadcastRequest",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "WarningType"},
                        "&criticality": "ignore",
                        "&id": "id-WarningType",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "WarningSecurityInfo"},
                        "&criticality": "ignore",
                        "&id": "id-WarningSecurityInfo",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "DataCodingScheme"},
                        "&criticality": "ignore",
                        "&id": "id-DataCodingScheme",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "WarningMessageContents"},
                        "&criticality": "ignore",
                        "&id": "id-WarningMessageContents",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "ConcurrentWarningMessageIndicator"},
                        "&criticality": "reject",
                        "&id": "id-ConcurrentWarningMessageIndicator",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
            "WriteReplaceWarningResponseIEs": {
                "class": "S1AP-PROTOCOL-IES",
                "members": [
                    {
                        "&Value": {"type": "MessageIdentifier"},
                        "&criticality": "reject",
                        "&id": "id-MessageIdentifier",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "SerialNumber"},
                        "&criticality": "reject",
                        "&id": "id-SerialNumber",
                        "&presence": "mandatory",
                    },
                    {
                        "&Value": {"type": "BroadcastCompletedAreaList"},
                        "&criticality": "ignore",
                        "&id": "id-BroadcastCompletedAreaList",
                        "&presence": "optional",
                    },
                    {
                        "&Value": {"type": "CriticalityDiagnostics"},
                        "&criticality": "ignore",
                        "&id": "id-CriticalityDiagnostics",
                        "&presence": "optional",
                    },
                    None,
                ],
            },
        },
        "tags": "AUTOMATIC",
        "types": {
            "CSGMembershipInfo": {
                "members": [
                    {"name": "cSGMembershipStatus", "type": "CSGMembershipStatus"},
                    {"name": "cSG-Id", "type": "CSG-Id"},
                    {
                        "name": "cellAccessMode",
                        "optional": True,
                        "type": "CellAccessMode",
                    },
                    {"name": "pLMNidentity", "optional": True, "type": "PLMNidentity"},
                    {
                        "actual-parameters": [
                            {"object-set": ["CSGMembershipInfo-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "CellTrafficTrace": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["CellTrafficTraceIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ConnectionEstablishmentIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ConnectionEstablishmentIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DeactivateTrace": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["DeactivateTraceIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DownlinkNASTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["DownlinkNASTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DownlinkNonUEAssociatedLPPaTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["DownlinkNonUEAssociatedLPPaTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DownlinkS1cdma2000tunnelling": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["DownlinkS1cdma2000tunnellingIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "DownlinkUEAssociatedLPPaTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["DownlinkUEAssociatedLPPaTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RAB-IE-ContainerList": {
                "actual-parameters": [
                    1,
                    "maxnoofE-RABs",
                    {"object-set": ["IEsSetParam"]},
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES"}],
                "parameters": ["IEsSetParam"],
                "type": "ProtocolIE-ContainerList",
            },
            "E-RAB-IE-ContainerPairList": {
                "actual-parameters": [
                    1,
                    "maxnoofE-RABs",
                    {"object-set": ["IEsSetParam"]},
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES-PAIR"}],
                "parameters": ["IEsSetParam"],
                "type": "ProtocolIE-ContainerPairList",
            },
            "E-RABAdmittedItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "name": "dL-transportLayerAddress",
                        "optional": True,
                        "type": "TransportLayerAddress",
                    },
                    {"name": "dL-gTP-TEID", "optional": True, "type": "GTP-TEID"},
                    {
                        "name": "uL-TransportLayerAddress",
                        "optional": True,
                        "type": "TransportLayerAddress",
                    },
                    {"name": "uL-GTP-TEID", "optional": True, "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABAdmittedItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABAdmittedList": {
                "actual-parameters": [{"object-set": ["E-RABAdmittedItemIEs"]}],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABDataForwardingItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "name": "dL-transportLayerAddress",
                        "optional": True,
                        "type": "TransportLayerAddress",
                    },
                    {"name": "dL-gTP-TEID", "optional": True, "type": "GTP-TEID"},
                    {
                        "name": "uL-TransportLayerAddress",
                        "optional": True,
                        "type": "TransportLayerAddress",
                    },
                    {"name": "uL-GTP-TEID", "optional": True, "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABDataForwardingItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABFailedToResumeItemResumeReq": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "cause", "type": "Cause"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABFailedToResumeItemResumeReq-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABFailedToResumeItemResumeRes": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "cause", "type": "Cause"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABFailedToResumeItemResumeRes-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABFailedToResumeListResumeReq": {
                "actual-parameters": [
                    {"object-set": ["E-RABFailedToResumeItemResumeReqIEs"]}
                ],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABFailedToResumeListResumeRes": {
                "actual-parameters": [
                    {"object-set": ["E-RABFailedToResumeItemResumeResIEs"]}
                ],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABFailedToSetupItemHOReqAck": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "cause", "type": "Cause"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABFailedToSetupItemHOReqAckExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABFailedtoSetupListHOReqAck": {
                "actual-parameters": [
                    {"object-set": ["E-RABFailedtoSetupItemHOReqAckIEs"]}
                ],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABModificationConfirm": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModificationConfirmIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABModificationIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModificationIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABModifyItemBearerModConf": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModifyItemBearerModConfExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABModifyItemBearerModRes": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModifyItemBearerModResExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABModifyListBearerModConf": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABModifyItemBearerModConfIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABModifyListBearerModRes": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABModifyItemBearerModResIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABModifyRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModifyRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABModifyResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABModifyResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABNotToBeModifiedItemBearerModInd": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "dL-GTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {
                                "object-set": [
                                    "E-RABNotToBeModifiedItemBearerModInd-ExtIEs"
                                ]
                            }
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABNotToBeModifiedListBearerModInd": {
                "actual-parameters": [
                    {"object-set": ["E-RABNotToBeModifiedItemBearerModIndIEs"]}
                ],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABReleaseCommand": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABReleaseCommandIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABReleaseIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABReleaseIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABReleaseItemBearerRelComp": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABReleaseItemBearerRelCompExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABReleaseListBearerRelComp": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABReleaseItemBearerRelCompIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABReleaseResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABReleaseResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABSetupItemBearerSURes": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABSetupItemBearerSUResExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABSetupItemCtxtSURes": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABSetupItemCtxtSUResExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABSetupListBearerSURes": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABSetupItemBearerSUResIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABSetupListCtxtSURes": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABSetupItemCtxtSUResIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABSetupRequest": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["E-RABSetupRequestIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABSetupResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABSetupResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABSubjecttoDataForwardingList": {
                "actual-parameters": [{"object-set": ["E-RABDataForwardingItemIEs"]}],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABToBeModifiedItemBearerModInd": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "dL-GTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeModifiedItemBearerModInd-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeModifiedItemBearerModReq": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "name": "e-RABLevelQoSParameters",
                        "type": "E-RABLevelQoSParameters",
                    },
                    {"name": "nAS-PDU", "type": "NAS-PDU"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeModifyItemBearerModReqExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeModifiedListBearerModInd": {
                "actual-parameters": [
                    {"object-set": ["E-RABToBeModifiedItemBearerModIndIEs"]}
                ],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABToBeModifiedListBearerModReq": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABToBeModifiedItemBearerModReqIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABToBeSetupItemBearerSUReq": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "name": "e-RABlevelQoSParameters",
                        "type": "E-RABLevelQoSParameters",
                    },
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {"name": "nAS-PDU", "type": "NAS-PDU"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeSetupItemBearerSUReqExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeSetupItemCtxtSUReq": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {
                        "name": "e-RABlevelQoSParameters",
                        "type": "E-RABLevelQoSParameters",
                    },
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {"name": "nAS-PDU", "optional": True, "type": "NAS-PDU"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeSetupItemCtxtSUReqExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeSetupItemHOReq": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "name": "e-RABlevelQosParameters",
                        "type": "E-RABLevelQoSParameters",
                    },
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeSetupItemHOReq-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeSetupListBearerSUReq": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABToBeSetupItemBearerSUReqIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABToBeSetupListCtxtSUReq": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["E-RABToBeSetupItemCtxtSUReqIEs"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofE-RABs")],
                "type": "SEQUENCE OF",
            },
            "E-RABToBeSetupListHOReq": {
                "actual-parameters": [{"object-set": ["E-RABToBeSetupItemHOReqIEs"]}],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABToBeSwitchedDLItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeSwitchedDLItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeSwitchedDLList": {
                "actual-parameters": [{"object-set": ["E-RABToBeSwitchedDLItemIEs"]}],
                "type": "E-RAB-IE-ContainerList",
            },
            "E-RABToBeSwitchedULItem": {
                "members": [
                    {"name": "e-RAB-ID", "type": "E-RAB-ID"},
                    {"name": "transportLayerAddress", "type": "TransportLayerAddress"},
                    {"name": "gTP-TEID", "type": "GTP-TEID"},
                    {
                        "actual-parameters": [
                            {"object-set": ["E-RABToBeSwitchedULItem-ExtIEs"]}
                        ],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "E-RABToBeSwitchedULList": {
                "actual-parameters": [{"object-set": ["E-RABToBeSwitchedULItemIEs"]}],
                "type": "E-RAB-IE-ContainerList",
            },
            "ENBCPRelocationIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBCPRelocationIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBConfigurationTransfer": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBConfigurationTransferIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBConfigurationUpdate": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBConfigurationUpdateIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBConfigurationUpdateAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBConfigurationUpdateAcknowledgeIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBConfigurationUpdateFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBConfigurationUpdateFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBDirectInformationTransfer": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["ENBDirectInformationTransferIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ENBStatusTransfer": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["ENBStatusTransferIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ErrorIndication": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["ErrorIndicationIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverCancel": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverCancelIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverCancelAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["HandoverCancelAcknowledgeIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverCommand": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverCommandIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverFailure": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverFailureIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverNotify": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverNotifyIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverPreparationFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["HandoverPreparationFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverRequest": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverRequestIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverRequestAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["HandoverRequestAcknowledgeIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "HandoverRequired": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["HandoverRequiredIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "InitialContextSetupFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["InitialContextSetupFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "InitialContextSetupRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["InitialContextSetupRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "InitialContextSetupResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["InitialContextSetupResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "InitialUEMessage": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["InitialUEMessage-IEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Inter-SystemInformationTransferType": {
                "members": [{"name": "rIMTransfer", "type": "RIMTransfer"}, None],
                "type": "CHOICE",
            },
            "KillRequest": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["KillRequestIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "KillResponse": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["KillResponseIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LocationReport": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["LocationReportIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LocationReportingControl": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["LocationReportingControlIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "LocationReportingFailureIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["LocationReportingFailureIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMECPRelocationIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMECPRelocationIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEConfigurationTransfer": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMEConfigurationTransferIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEConfigurationUpdate": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMEConfigurationUpdateIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEConfigurationUpdateAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMEConfigurationUpdateAcknowledgeIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEConfigurationUpdateFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMEConfigurationUpdateFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEDirectInformationTransfer": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["MMEDirectInformationTransferIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "MMEStatusTransfer": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["MMEStatusTransferIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "NASDeliveryIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["NASDeliveryIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "NASNonDeliveryIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["NASNonDeliveryIndication-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "OverloadStart": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["OverloadStartIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "OverloadStop": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["OverloadStopIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PWSFailureIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["PWSFailureIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PWSRestartIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["PWSRestartIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Paging": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["PagingIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PathSwitchRequest": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["PathSwitchRequestIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PathSwitchRequestAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["PathSwitchRequestAcknowledgeIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PathSwitchRequestFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["PathSwitchRequestFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "PrivateMessage": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["PrivateMessageIEs"]}],
                        "name": "privateIEs",
                        "type": "PrivateIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ProtocolError-IE-ContainerList": {
                "actual-parameters": [
                    1,
                    "maxnoofE-RABs",
                    {"object-set": ["IEsSetParam"]},
                ],
                "parameter-governors": [{"type": "S1AP-PROTOCOL-IES"}],
                "parameters": ["IEsSetParam"],
                "type": "ProtocolIE-ContainerList",
            },
            "RerouteNASRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["RerouteNASRequest-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "Reset": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["ResetIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ResetAcknowledge": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["ResetAcknowledgeIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "ResetAll": {"type": "ENUMERATED", "values": [("reset-all", 0), None]},
            "ResetType": {
                "members": [
                    {"name": "s1-Interface", "type": "ResetAll"},
                    {
                        "name": "partOfS1-Interface",
                        "type": "UE-associatedLogicalS1-ConnectionListRes",
                    },
                    None,
                ],
                "type": "CHOICE",
            },
            "RetrieveUEInformation": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["RetrieveUEInformationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "S1SetupFailure": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["S1SetupFailureIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "S1SetupRequest": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["S1SetupRequestIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "S1SetupResponse": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["S1SetupResponseIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAIItem": {
                "members": [
                    {"name": "tAI", "type": "TAI"},
                    {
                        "actual-parameters": [{"object-set": ["TAIItemExtIEs"]}],
                        "name": "iE-Extensions",
                        "optional": True,
                        "type": "ProtocolExtensionContainer",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TAIList": {
                "element": {
                    "actual-parameters": [{"object-set": ["TAIItemIEs"]}],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofTAIs")],
                "type": "SEQUENCE OF",
            },
            "TraceFailureIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["TraceFailureIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "TraceStart": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["TraceStartIEs"]}],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UE-associatedLogicalS1-ConnectionListRes": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["UE-associatedLogicalS1-ConnectionItemRes"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofIndividualS1ConnectionsToReset")],
                "type": "SEQUENCE OF",
            },
            "UE-associatedLogicalS1-ConnectionListResAck": {
                "element": {
                    "actual-parameters": [
                        {"object-set": ["UE-associatedLogicalS1-ConnectionItemResAck"]}
                    ],
                    "type": "ProtocolIE-SingleContainer",
                },
                "size": [(1, "maxnoofIndividualS1ConnectionsToReset")],
                "type": "SEQUENCE OF",
            },
            "UECapabilityInfoIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UECapabilityInfoIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextModificationConfirm": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextModificationConfirmIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextModificationFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextModificationFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextModificationIndication": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextModificationIndicationIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextModificationRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextModificationRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextModificationResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextModificationResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextReleaseCommand": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextReleaseCommand-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextReleaseComplete": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextReleaseComplete-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextReleaseRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextReleaseRequest-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextResumeFailure": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextResumeFailureIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextResumeRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextResumeRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextResumeResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextResumeResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextSuspendRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextSuspendRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEContextSuspendResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEContextSuspendResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UEInformationTransfer": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UEInformationTransferIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UERadioCapabilityMatchRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UERadioCapabilityMatchRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UERadioCapabilityMatchResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UERadioCapabilityMatchResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UplinkNASTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UplinkNASTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UplinkNonUEAssociatedLPPaTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UplinkNonUEAssociatedLPPaTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UplinkS1cdma2000tunnelling": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UplinkS1cdma2000tunnellingIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "UplinkUEAssociatedLPPaTransport": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["UplinkUEAssociatedLPPaTransport-IEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "WriteReplaceWarningRequest": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["WriteReplaceWarningRequestIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
            "WriteReplaceWarningResponse": {
                "members": [
                    {
                        "actual-parameters": [
                            {"object-set": ["WriteReplaceWarningResponseIEs"]}
                        ],
                        "name": "protocolIEs",
                        "type": "ProtocolIE-Container",
                    },
                    None,
                ],
                "type": "SEQUENCE",
            },
        },
        "values": {},
    },
    "S1AP-PDU-Descriptions": {
        "extensibility-implied": False,
        "imports": {
            "S1AP-CommonDataTypes": ["Criticality", "ProcedureCode"],
            "S1AP-Constants": [
                "id-CellTrafficTrace",
                "id-ConnectionEstablishmentIndication",
                "id-DeactivateTrace",
                "id-DownlinkS1cdma2000tunnelling",
                "id-E-RABModificationIndication",
                "id-E-RABModify",
                "id-E-RABRelease",
                "id-E-RABReleaseIndication",
                "id-E-RABSetup",
                "id-ENBConfigurationUpdate",
                "id-ErrorIndication",
                "id-HandoverCancel",
                "id-HandoverNotification",
                "id-HandoverPreparation",
                "id-HandoverResourceAllocation",
                "id-InitialContextSetup",
                "id-Kill",
                "id-LocationReport",
                "id-LocationReportingControl",
                "id-LocationReportingFailureIndication",
                "id-MMECPRelocationIndication",
                "id-MMEConfigurationTransfer",
                "id-MMEConfigurationUpdate",
                "id-MMEDirectInformationTransfer",
                "id-MMEStatusTransfer",
                "id-NASDeliveryIndication",
                "id-NASNonDeliveryIndication",
                "id-OverloadStart",
                "id-OverloadStop",
                "id-PWSFailureIndication",
                "id-PWSRestartIndication",
                "id-Paging",
                "id-PathSwitchRequest",
                "id-PrivateMessage",
                "id-RerouteNASRequest",
                "id-Reset",
                "id-RetrieveUEInformation",
                "id-S1Setup",
                "id-TraceFailureIndication",
                "id-TraceStart",
                "id-UECapabilityInfoIndication",
                "id-UEContextModification",
                "id-UEContextModificationIndication",
                "id-UEContextRelease",
                "id-UEContextReleaseRequest",
                "id-UEContextResume",
                "id-UEContextSuspend",
                "id-UEInformationTransfer",
                "id-UERadioCapabilityMatch",
                "id-UplinkS1cdma2000tunnelling",
                "id-WriteReplaceWarning",
                "id-downlinkNASTransport",
                "id-downlinkNonUEAssociatedLPPaTransport",
                "id-downlinkUEAssociatedLPPaTransport",
                "id-eNBCPRelocationIndication",
                "id-eNBConfigurationTransfer",
                "id-eNBDirectInformationTransfer",
                "id-eNBStatusTransfer",
                "id-initialUEMessage",
                "id-uplinkNASTransport",
                "id-uplinkNonUEAssociatedLPPaTransport",
                "id-uplinkUEAssociatedLPPaTransport",
            ],
            "S1AP-PDU-Contents": [
                "CellTrafficTrace",
                "ConnectionEstablishmentIndication",
                "DeactivateTrace",
                "DownlinkNASTransport",
                "DownlinkNonUEAssociatedLPPaTransport",
                "DownlinkS1cdma2000tunnelling",
                "DownlinkUEAssociatedLPPaTransport",
                "E-RABModificationConfirm",
                "E-RABModificationIndication",
                "E-RABModifyRequest",
                "E-RABModifyResponse",
                "E-RABReleaseCommand",
                "E-RABReleaseIndication",
                "E-RABReleaseResponse",
                "E-RABSetupRequest",
                "E-RABSetupResponse",
                "ENBCPRelocationIndication",
                "ENBConfigurationTransfer",
                "ENBConfigurationUpdate",
                "ENBConfigurationUpdateAcknowledge",
                "ENBConfigurationUpdateFailure",
                "ENBDirectInformationTransfer",
                "ENBStatusTransfer",
                "ErrorIndication",
                "HandoverCancel",
                "HandoverCancelAcknowledge",
                "HandoverCommand",
                "HandoverFailure",
                "HandoverNotify",
                "HandoverPreparationFailure",
                "HandoverRequest",
                "HandoverRequestAcknowledge",
                "HandoverRequired",
                "InitialContextSetupFailure",
                "InitialContextSetupRequest",
                "InitialContextSetupResponse",
                "InitialUEMessage",
                "KillRequest",
                "KillResponse",
                "LocationReport",
                "LocationReportingControl",
                "LocationReportingFailureIndication",
                "MMECPRelocationIndication",
                "MMEConfigurationTransfer",
                "MMEConfigurationUpdate",
                "MMEConfigurationUpdateAcknowledge",
                "MMEConfigurationUpdateFailure",
                "MMEDirectInformationTransfer",
                "MMEStatusTransfer",
                "NASDeliveryIndication",
                "NASNonDeliveryIndication",
                "OverloadStart",
                "OverloadStop",
                "PWSFailureIndication",
                "PWSRestartIndication",
                "Paging",
                "PathSwitchRequest",
                "PathSwitchRequestAcknowledge",
                "PathSwitchRequestFailure",
                "PrivateMessage",
                "RerouteNASRequest",
                "Reset",
                "ResetAcknowledge",
                "RetrieveUEInformation",
                "S1SetupFailure",
                "S1SetupRequest",
                "S1SetupResponse",
                "TraceFailureIndication",
                "TraceStart",
                "UECapabilityInfoIndication",
                "UEContextModificationConfirm",
                "UEContextModificationFailure",
                "UEContextModificationIndication",
                "UEContextModificationRequest",
                "UEContextModificationResponse",
                "UEContextReleaseCommand",
                "UEContextReleaseComplete",
                "UEContextReleaseRequest",
                "UEContextResumeFailure",
                "UEContextResumeRequest",
                "UEContextResumeResponse",
                "UEContextSuspendRequest",
                "UEContextSuspendResponse",
                "UEInformationTransfer",
                "UERadioCapabilityMatchRequest",
                "UERadioCapabilityMatchResponse",
                "UplinkNASTransport",
                "UplinkNonUEAssociatedLPPaTransport",
                "UplinkS1cdma2000tunnelling",
                "UplinkUEAssociatedLPPaTransport",
                "WriteReplaceWarningRequest",
                "WriteReplaceWarningResponse",
            ],
        },
        "object-classes": {
            "S1AP-ELEMENTARY-PROCEDURE": {
                "members": [
                    {"name": "&InitiatingMessage", "type": "OpenType"},
                    {"name": "&SuccessfulOutcome", "type": "OpenType"},
                    {"name": "&UnsuccessfulOutcome", "type": "OpenType"},
                    {"name": "&procedureCode", "type": "ProcedureCode"},
                    {"name": "&criticality", "type": "Criticality"},
                ],
                "syntax": [
                    "INITIATING",
                    "MESSAGE",
                    "&InitiatingMessage",
                    ["SUCCESSFUL", "OUTCOME", "&SuccessfulOutcome"],
                    ["UNSUCCESSFUL", "OUTCOME", "&UnsuccessfulOutcome"],
                    "PROCEDURE",
                    "CODE",
                    "&procedureCode",
                    ["CRITICALITY", "&criticality"],
                ],
            }
        },
        "object-sets": {
            "S1AP-ELEMENTARY-PROCEDURES": {
                "class": "S1AP-ELEMENTARY-PROCEDURE",
                "members": [
                    "S1AP-ELEMENTARY-PROCEDURES-CLASS-1",
                    "S1AP-ELEMENTARY-PROCEDURES-CLASS-2",
                    None,
                ],
            },
            "S1AP-ELEMENTARY-PROCEDURES-CLASS-1": {
                "class": "S1AP-ELEMENTARY-PROCEDURE",
                "members": [
                    "handoverPreparation",
                    "handoverResourceAllocation",
                    "pathSwitchRequest",
                    "e-RABSetup",
                    "e-RABModify",
                    "e-RABRelease",
                    "initialContextSetup",
                    "handoverCancel",
                    "kill",
                    "reset",
                    "s1Setup",
                    "uEContextModification",
                    "uEContextRelease",
                    "eNBConfigurationUpdate",
                    "mMEConfigurationUpdate",
                    "writeReplaceWarning",
                    None,
                    "uERadioCapabilityMatch",
                    "e-RABModificationIndication",
                    "uEContextModificationIndication",
                    "uEContextSuspend",
                    "uEContextResume",
                ],
            },
            "S1AP-ELEMENTARY-PROCEDURES-CLASS-2": {
                "class": "S1AP-ELEMENTARY-PROCEDURE",
                "members": [
                    "handoverNotification",
                    "e-RABReleaseIndication",
                    "paging",
                    "downlinkNASTransport",
                    "initialUEMessage",
                    "uplinkNASTransport",
                    "errorIndication",
                    "nASNonDeliveryIndication",
                    "uEContextReleaseRequest",
                    "downlinkS1cdma2000tunnelling",
                    "uplinkS1cdma2000tunnelling",
                    "uECapabilityInfoIndication",
                    "eNBStatusTransfer",
                    "mMEStatusTransfer",
                    "deactivateTrace",
                    "traceStart",
                    "traceFailureIndication",
                    "cellTrafficTrace",
                    "locationReportingControl",
                    "locationReportingFailureIndication",
                    "locationReport",
                    "overloadStart",
                    "overloadStop",
                    "eNBDirectInformationTransfer",
                    "mMEDirectInformationTransfer",
                    "eNBConfigurationTransfer",
                    "mMEConfigurationTransfer",
                    "privateMessage",
                    None,
                    "downlinkUEAssociatedLPPaTransport",
                    "uplinkUEAssociatedLPPaTransport",
                    "downlinkNonUEAssociatedLPPaTransport",
                    "uplinkNonUEAssociatedLPPaTransport",
                    "pWSRestartIndication",
                    "rerouteNASRequest",
                    "pWSFailureIndication",
                    "connectionEstablishmentIndication",
                    "nASDeliveryIndication",
                    "retrieveUEInformation",
                    "uEInformationTransfer",
                    "eNBCPRelocationIndication",
                    "mMECPRelocationIndication",
                ],
            },
        },
        "tags": "AUTOMATIC",
        "types": {
            "InitiatingMessage": {
                "members": [
                    {
                        "name": "procedureCode",
                        "table": {"type": "S1AP-ELEMENTARY-PROCEDURES"},
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&procedureCode",
                    },
                    {
                        "name": "criticality",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&criticality",
                    },
                    {
                        "name": "value",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&InitiatingMessage",
                    },
                ],
                "type": "SEQUENCE",
            },
            "S1AP-PDU": {
                "members": [
                    {"name": "initiatingMessage", "type": "InitiatingMessage"},
                    {"name": "successfulOutcome", "type": "SuccessfulOutcome"},
                    {"name": "unsuccessfulOutcome", "type": "UnsuccessfulOutcome"},
                    None,
                ],
                "type": "CHOICE",
            },
            "SuccessfulOutcome": {
                "members": [
                    {
                        "name": "procedureCode",
                        "table": {"type": "S1AP-ELEMENTARY-PROCEDURES"},
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&procedureCode",
                    },
                    {
                        "name": "criticality",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&criticality",
                    },
                    {
                        "name": "value",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&SuccessfulOutcome",
                    },
                ],
                "type": "SEQUENCE",
            },
            "UnsuccessfulOutcome": {
                "members": [
                    {
                        "name": "procedureCode",
                        "table": {"type": "S1AP-ELEMENTARY-PROCEDURES"},
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&procedureCode",
                    },
                    {
                        "name": "criticality",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&criticality",
                    },
                    {
                        "name": "value",
                        "table": ["S1AP-ELEMENTARY-PROCEDURES", ["procedureCode"]],
                        "type": "S1AP-ELEMENTARY-PROCEDURE.&UnsuccessfulOutcome",
                    },
                ],
                "type": "SEQUENCE",
            },
        },
        "values": {
            "cellTrafficTrace": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "CellTrafficTrace"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-CellTrafficTrace",
                },
            },
            "connectionEstablishmentIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ConnectionEstablishmentIndication"},
                    "&criticality": "reject",
                    "&procedureCode": "id-ConnectionEstablishmentIndication",
                },
            },
            "deactivateTrace": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "DeactivateTrace"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-DeactivateTrace",
                },
            },
            "downlinkNASTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "DownlinkNASTransport"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-downlinkNASTransport",
                },
            },
            "downlinkNonUEAssociatedLPPaTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {
                        "type": "DownlinkNonUEAssociatedLPPaTransport"
                    },
                    "&criticality": "ignore",
                    "&procedureCode": "id-downlinkNonUEAssociatedLPPaTransport",
                },
            },
            "downlinkS1cdma2000tunnelling": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "DownlinkS1cdma2000tunnelling"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-DownlinkS1cdma2000tunnelling",
                },
            },
            "downlinkUEAssociatedLPPaTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "DownlinkUEAssociatedLPPaTransport"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-downlinkUEAssociatedLPPaTransport",
                },
            },
            "e-RABModificationIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "E-RABModificationIndication"},
                    "&SuccessfulOutcome": {"type": "E-RABModificationConfirm"},
                    "&criticality": "reject",
                    "&procedureCode": "id-E-RABModificationIndication",
                },
            },
            "e-RABModify": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "E-RABModifyRequest"},
                    "&SuccessfulOutcome": {"type": "E-RABModifyResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-E-RABModify",
                },
            },
            "e-RABRelease": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "E-RABReleaseCommand"},
                    "&SuccessfulOutcome": {"type": "E-RABReleaseResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-E-RABRelease",
                },
            },
            "e-RABReleaseIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "E-RABReleaseIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-E-RABReleaseIndication",
                },
            },
            "e-RABSetup": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "E-RABSetupRequest"},
                    "&SuccessfulOutcome": {"type": "E-RABSetupResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-E-RABSetup",
                },
            },
            "eNBCPRelocationIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ENBCPRelocationIndication"},
                    "&criticality": "reject",
                    "&procedureCode": "id-eNBCPRelocationIndication",
                },
            },
            "eNBConfigurationTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ENBConfigurationTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-eNBConfigurationTransfer",
                },
            },
            "eNBConfigurationUpdate": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ENBConfigurationUpdate"},
                    "&SuccessfulOutcome": {"type": "ENBConfigurationUpdateAcknowledge"},
                    "&UnsuccessfulOutcome": {"type": "ENBConfigurationUpdateFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-ENBConfigurationUpdate",
                },
            },
            "eNBDirectInformationTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ENBDirectInformationTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-eNBDirectInformationTransfer",
                },
            },
            "eNBStatusTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ENBStatusTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-eNBStatusTransfer",
                },
            },
            "errorIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "ErrorIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-ErrorIndication",
                },
            },
            "handoverCancel": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "HandoverCancel"},
                    "&SuccessfulOutcome": {"type": "HandoverCancelAcknowledge"},
                    "&criticality": "reject",
                    "&procedureCode": "id-HandoverCancel",
                },
            },
            "handoverNotification": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "HandoverNotify"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-HandoverNotification",
                },
            },
            "handoverPreparation": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "HandoverRequired"},
                    "&SuccessfulOutcome": {"type": "HandoverCommand"},
                    "&UnsuccessfulOutcome": {"type": "HandoverPreparationFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-HandoverPreparation",
                },
            },
            "handoverResourceAllocation": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "HandoverRequest"},
                    "&SuccessfulOutcome": {"type": "HandoverRequestAcknowledge"},
                    "&UnsuccessfulOutcome": {"type": "HandoverFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-HandoverResourceAllocation",
                },
            },
            "initialContextSetup": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "InitialContextSetupRequest"},
                    "&SuccessfulOutcome": {"type": "InitialContextSetupResponse"},
                    "&UnsuccessfulOutcome": {"type": "InitialContextSetupFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-InitialContextSetup",
                },
            },
            "initialUEMessage": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "InitialUEMessage"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-initialUEMessage",
                },
            },
            "kill": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "KillRequest"},
                    "&SuccessfulOutcome": {"type": "KillResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-Kill",
                },
            },
            "locationReport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "LocationReport"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-LocationReport",
                },
            },
            "locationReportingControl": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "LocationReportingControl"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-LocationReportingControl",
                },
            },
            "locationReportingFailureIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {
                        "type": "LocationReportingFailureIndication"
                    },
                    "&criticality": "ignore",
                    "&procedureCode": "id-LocationReportingFailureIndication",
                },
            },
            "mMECPRelocationIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "MMECPRelocationIndication"},
                    "&criticality": "reject",
                    "&procedureCode": "id-MMECPRelocationIndication",
                },
            },
            "mMEConfigurationTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "MMEConfigurationTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-MMEConfigurationTransfer",
                },
            },
            "mMEConfigurationUpdate": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "MMEConfigurationUpdate"},
                    "&SuccessfulOutcome": {"type": "MMEConfigurationUpdateAcknowledge"},
                    "&UnsuccessfulOutcome": {"type": "MMEConfigurationUpdateFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-MMEConfigurationUpdate",
                },
            },
            "mMEDirectInformationTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "MMEDirectInformationTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-MMEDirectInformationTransfer",
                },
            },
            "mMEStatusTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "MMEStatusTransfer"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-MMEStatusTransfer",
                },
            },
            "nASDeliveryIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "NASDeliveryIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-NASDeliveryIndication",
                },
            },
            "nASNonDeliveryIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "NASNonDeliveryIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-NASNonDeliveryIndication",
                },
            },
            "overloadStart": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "OverloadStart"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-OverloadStart",
                },
            },
            "overloadStop": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "OverloadStop"},
                    "&criticality": "reject",
                    "&procedureCode": "id-OverloadStop",
                },
            },
            "pWSFailureIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "PWSFailureIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-PWSFailureIndication",
                },
            },
            "pWSRestartIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "PWSRestartIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-PWSRestartIndication",
                },
            },
            "paging": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "Paging"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-Paging",
                },
            },
            "pathSwitchRequest": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "PathSwitchRequest"},
                    "&SuccessfulOutcome": {"type": "PathSwitchRequestAcknowledge"},
                    "&UnsuccessfulOutcome": {"type": "PathSwitchRequestFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-PathSwitchRequest",
                },
            },
            "privateMessage": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "PrivateMessage"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-PrivateMessage",
                },
            },
            "rerouteNASRequest": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "RerouteNASRequest"},
                    "&criticality": "reject",
                    "&procedureCode": "id-RerouteNASRequest",
                },
            },
            "reset": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "Reset"},
                    "&SuccessfulOutcome": {"type": "ResetAcknowledge"},
                    "&criticality": "reject",
                    "&procedureCode": "id-Reset",
                },
            },
            "retrieveUEInformation": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "RetrieveUEInformation"},
                    "&criticality": "reject",
                    "&procedureCode": "id-RetrieveUEInformation",
                },
            },
            "s1Setup": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "S1SetupRequest"},
                    "&SuccessfulOutcome": {"type": "S1SetupResponse"},
                    "&UnsuccessfulOutcome": {"type": "S1SetupFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-S1Setup",
                },
            },
            "traceFailureIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "TraceFailureIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-TraceFailureIndication",
                },
            },
            "traceStart": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "TraceStart"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-TraceStart",
                },
            },
            "uECapabilityInfoIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UECapabilityInfoIndication"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-UECapabilityInfoIndication",
                },
            },
            "uEContextModification": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextModificationRequest"},
                    "&SuccessfulOutcome": {"type": "UEContextModificationResponse"},
                    "&UnsuccessfulOutcome": {"type": "UEContextModificationFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEContextModification",
                },
            },
            "uEContextModificationIndication": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextModificationIndication"},
                    "&SuccessfulOutcome": {"type": "UEContextModificationConfirm"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEContextModificationIndication",
                },
            },
            "uEContextRelease": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextReleaseCommand"},
                    "&SuccessfulOutcome": {"type": "UEContextReleaseComplete"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEContextRelease",
                },
            },
            "uEContextReleaseRequest": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextReleaseRequest"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-UEContextReleaseRequest",
                },
            },
            "uEContextResume": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextResumeRequest"},
                    "&SuccessfulOutcome": {"type": "UEContextResumeResponse"},
                    "&UnsuccessfulOutcome": {"type": "UEContextResumeFailure"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEContextResume",
                },
            },
            "uEContextSuspend": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEContextSuspendRequest"},
                    "&SuccessfulOutcome": {"type": "UEContextSuspendResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEContextSuspend",
                },
            },
            "uEInformationTransfer": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UEInformationTransfer"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UEInformationTransfer",
                },
            },
            "uERadioCapabilityMatch": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UERadioCapabilityMatchRequest"},
                    "&SuccessfulOutcome": {"type": "UERadioCapabilityMatchResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-UERadioCapabilityMatch",
                },
            },
            "uplinkNASTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UplinkNASTransport"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-uplinkNASTransport",
                },
            },
            "uplinkNonUEAssociatedLPPaTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {
                        "type": "UplinkNonUEAssociatedLPPaTransport"
                    },
                    "&criticality": "ignore",
                    "&procedureCode": "id-uplinkNonUEAssociatedLPPaTransport",
                },
            },
            "uplinkS1cdma2000tunnelling": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UplinkS1cdma2000tunnelling"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-UplinkS1cdma2000tunnelling",
                },
            },
            "uplinkUEAssociatedLPPaTransport": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "UplinkUEAssociatedLPPaTransport"},
                    "&criticality": "ignore",
                    "&procedureCode": "id-uplinkUEAssociatedLPPaTransport",
                },
            },
            "writeReplaceWarning": {
                "type": "S1AP-ELEMENTARY-PROCEDURE",
                "value": {
                    "&InitiatingMessage": {"type": "WriteReplaceWarningRequest"},
                    "&SuccessfulOutcome": {"type": "WriteReplaceWarningResponse"},
                    "&criticality": "reject",
                    "&procedureCode": "id-WriteReplaceWarning",
                },
            },
        },
    },
}
