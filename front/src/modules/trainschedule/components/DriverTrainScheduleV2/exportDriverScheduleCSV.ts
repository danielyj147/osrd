import * as d3 from 'd3';

import type {
  ElectrificationRangeV2,
  SimulationResponseSuccess,
} from 'applications/operationalStudies/types';
import { convertDepartureTimeIntoSec } from 'applications/operationalStudies/utils';
import type { ReportTrainV2, TrainScheduleBase } from 'common/api/osrdEditoastApi';
import type { PositionSpeedTime, SpeedPosition, Train } from 'reducers/osrdsimulation/types';
import { timestampToHHMMSS } from 'utils/date';
import { mmToM } from 'utils/physics';
import { ms2sec } from 'utils/timeManipulation';

import type { OperationalPointWithTimeAndSpeed } from './types';
import { interpolateValue } from './utils';
import { BaseOrEco, type BaseOrEcoType } from '../DriverTrainSchedule/consts';

/**
 * CSV Export of trainschedule
 *
 * Rows : position in km, speed in km/h, speed limit in km/h, time in s
 *
 */

enum CSVKeys {
  op = 'op',
  ch = 'ch',
  trackName = 'trackName',
  time = 'time',
  seconds = 'seconds',
  position = 'position',
  speed = 'speed',
  speedLimit = 'speedLimit',
  lineCode = 'lineCode',
  electrificationType = 'electrificationType',
  electrificationMode = 'electrificationMode',
  electrificationProfile = 'electrificationProfile',
}

type CSVData = {
  [key in keyof typeof CSVKeys]?: string;
};

type PositionSpeedTimeOP = PositionSpeedTime & {
  speedLimit?: number;
  op?: string;
  ch?: string;
  lineCode?: string;
  trackName?: string;
  electrificationType?: string;
  electrificationMode?: string;
  electrificationProfile?: string;
};

const pointToComma = (number: number) => number.toString().replace('.', ',');
const compareOldActualValues = (old?: string, actual?: string) =>
  old !== undefined && actual === undefined ? old : actual;

const getStepSpeedLimit = (_position: number, speedLimitList: Train['vmax']) => {
  const bisector = d3.bisectLeft(
    speedLimitList.map((d: SpeedPosition) => d.position),
    _position
  );
  return speedLimitList[bisector].speed || 0;
};

// Add OPs inside speedsteps array, gather speedlimit with stop position, add electrification ranges,
// and sort the array along position before return
const overloadSteps = (
  trainRegime: ReportTrainV2,
  operationalPoints: OperationalPointWithTimeAndSpeed[],
  speedLimits: SpeedPosition[],
  electrificationRanges: ElectrificationRangeV2[]
): PositionSpeedTimeOP[] => {
  const speedsAtOps = operationalPoints.map((op) => ({
    position: op.position,
    speed: op.speed,
    time: op.time,
    op: op.name,
    ch: op.ch,
    lineCode: op.line_code,
    trackName: op.track_name,
  }));
  const speedsAtSpeedLimitChange = speedLimits.map((speedLimit) => ({
    position: mmToM(speedLimit.position),
    speed: interpolateValue(trainRegime, speedLimit.position, 'speeds'),
    speedLimit: speedLimit.speed,
    time: interpolateValue(trainRegime, speedLimit.position, 'times'),
  }));
  const speedsAtElectrificationRanges: PositionSpeedTimeOP[] = [];
  electrificationRanges.forEach((electrification, idx) => {
    const electrificationType = electrification.electrificationUsage.type;
    const electricalProfileType = electrification.electrificationUsage.electrical_profile_type;

    const electrificationMode =
      electrificationType === 'electrification' ? electrification.electrificationUsage.voltage : '';
    const electrificationProfile =
      electrificationType === 'electrification' &&
      electricalProfileType === 'profile' &&
      electrification.electrificationUsage.profile
        ? electrification.electrificationUsage.profile
        : '';
    const electrificationStart = electrification.start;

    speedsAtElectrificationRanges.push({
      position: mmToM(electrificationStart),
      speed: interpolateValue(trainRegime, electrificationStart, 'speeds'),
      electrificationType,
      electrificationMode,
      electrificationProfile,
      time: interpolateValue(trainRegime, electrificationStart, 'times'),
    });

    // ElectrificationRanges could not be continuous, so we've to handle the case of
    // empty ranges: when previousStop isn't equal to nextStart, we add a one-meter away
    // step with empty values to ensure correct spread of information along all steps
    if (
      electrificationRanges[idx + 1] &&
      electrification.stop < electrificationRanges[idx + 1].start
    ) {
      speedsAtElectrificationRanges.push({
        position: mmToM(electrification.stop + 1),
        speed: interpolateValue(trainRegime, electrification.stop + 1, 'speeds'),
        electrificationType: '',
        electrificationMode: '',
        electrificationProfile: '',
        time: interpolateValue(trainRegime, electrification.stop + 1, 'times'),
      });
    }
  });

  const formattedTrainRegime = trainRegime.speeds.map((speed, index) => ({
    speed,
    position: mmToM(trainRegime.positions[index]),
    time: trainRegime.times[index],
  }));

  const speedsWithOPsAndSpeedLimits = formattedTrainRegime.concat(
    speedsAtOps,
    speedsAtSpeedLimitChange,
    speedsAtElectrificationRanges
  );

  return speedsWithOPsAndSpeedLimits.sort((stepA, stepB) => stepA.position - stepB.position);
};

// Complete empty cells of data between steps for specific columns
function spreadDataBetweenSteps(steps: CSVData[]): CSVData[] {
  let oldTrackName: string | undefined;
  let oldLineCode: string | undefined;
  let oldElectrificationType: string | undefined;
  let odlElectrificationMode: string | undefined;
  let oldElectrificationProfile: string | undefined;
  const newSteps: CSVData[] = [];
  steps.forEach((step) => {
    const newTrackName = compareOldActualValues(oldTrackName, step.trackName);
    const newLineCode = compareOldActualValues(oldLineCode, step.lineCode);
    const newElectrificationType = compareOldActualValues(
      oldElectrificationType,
      step.electrificationType
    );
    const newElectrificationMode = compareOldActualValues(
      odlElectrificationMode,
      step.electrificationMode
    );
    const newElectrificationProfile = compareOldActualValues(
      oldElectrificationProfile,
      step.electrificationProfile
    );
    newSteps.push({
      ...step,
      trackName: newTrackName,
      lineCode: newLineCode,
      electrificationType: newElectrificationType,
      electrificationMode: newElectrificationMode,
      electrificationProfile: newElectrificationProfile,
    });
    oldTrackName = newTrackName;
    oldLineCode = newLineCode;
    oldElectrificationType = newElectrificationType;
    odlElectrificationMode = newElectrificationMode;
    oldElectrificationProfile = newElectrificationProfile;
  });
  return newSteps;
}

function createFakeLinkWithData(trainName: string, baseOrEco: BaseOrEcoType, csvData: CSVData[]) {
  const currentDate = new Date();
  const header = `Date: ${currentDate.toLocaleString()}\nName: ${trainName}\nType:${baseOrEco}\n`;
  const keyLine = `${Object.values(CSVKeys).join(';')}\n`;
  const csvContent = `data:text/csv;charset=utf-8,${header}\n${keyLine}${csvData.map((obj) => Object.values(obj).join(';')).join('\n')}`;
  const encodedUri = encodeURI(csvContent);
  const fakeLink = document.createElement('a');
  fakeLink.setAttribute('href', encodedUri);
  fakeLink.setAttribute('download', `export-${trainName}-${baseOrEco}.csv`);
  document.body.appendChild(fakeLink);
  fakeLink.click();
  document.body.removeChild(fakeLink);
}

export default function driverTrainScheduleV2ExportCSV(
  simulatedTrain: SimulationResponseSuccess,
  operationalPoints: OperationalPointWithTimeAndSpeed[],
  electrificationRanges: ElectrificationRangeV2[],
  baseOrEco: BaseOrEcoType,
  train: TrainScheduleBase
) {
  const trainRegime =
    baseOrEco === BaseOrEco.base ? simulatedTrain.base : simulatedTrain.final_output;
  const trainRegimeWithAccurateTime: ReportTrainV2 = {
    ...trainRegime,
    times: trainRegime.times.map(
      (time) => convertDepartureTimeIntoSec(train.start_time) + ms2sec(time)
    ),
  };
  const formattedMrsp = simulatedTrain.mrsp.positions.map((_position, index) => ({
    position: _position,
    speed: simulatedTrain.mrsp.speeds[index],
  }));

  const speedsWithOPsAndSpeedLimits = overloadSteps(
    trainRegimeWithAccurateTime,
    operationalPoints,
    formattedMrsp,
    electrificationRanges
  );

  const steps = speedsWithOPsAndSpeedLimits.map((speed) => ({
    op: speed.op || '',
    ch: speed.ch || '',
    trackName: speed.trackName,
    time: timestampToHHMMSS(speed.time),
    seconds: pointToComma(+speed.time.toFixed(1)),
    position: pointToComma(+(speed.position / 1000).toFixed(3)),
    speed: pointToComma(+(speed.speed * 3.6).toFixed(3)),
    speedLimit: pointToComma(
      Math.round((speed.speedLimit ?? getStepSpeedLimit(speed.position, formattedMrsp)) * 3.6)
    ),
    lineCode: speed.lineCode,
    electrificationType: speed.electrificationType,
    electrificationMode: speed.electrificationMode,
    electrificationProfile: speed.electrificationProfile,
  }));
  if (steps) createFakeLinkWithData(train.train_name, baseOrEco, spreadDataBetweenSteps(steps));
}
